use notify::{Event, RecursiveMode, Result, Watcher};
use std::collections::{HashMap, HashSet};
use std::env::current_dir;
use std::fs::{self, canonicalize, File};
use std::io::{self, BufRead, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc;
use walkdir::WalkDir;

use clap::Parser;

/// A simple include preprocessor
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    ///  Watch for changes in the source directory
    #[arg(short, long, default_value_t = false)]
    watch: bool,

    /// Source directory
    #[arg(short, long, default_value = ".")]
    src: String,

    /// Target directory
    #[arg(short, long, default_value = "target")]
    target: String,

    /// Include Prefix
    #[arg(short, long, default_value = "--include")]
    include: String,

    ///Verbose output - prints the input and output file paths
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let src = if args.src == "." {
        &current_dir().unwrap()
    } else {
        Path::new(&args.src)
    };
    let target = if args.target == "." {
        &current_dir().unwrap()
    } else {
        Path::new(&args.target)
    };

    if !target.exists() {
        let res = fs::create_dir_all(target);
        if res.is_err() {
            eprintln!(
                "The target directory {:?} does not exist and could not be created.",
                target
            );
            return Err(res.err().unwrap().into());
        }
    }

    let abs_src = fs::canonicalize(src)?;
    let abs_target = fs::canonicalize(target)?;

    let mut included_files: HashMap<PathBuf, HashSet<PathBuf>> = HashMap::new();

    for file in list_of_paths(&abs_src, &abs_target)? {
        match process_file(
            &file,
            &target.join(file.clone().strip_prefix(&abs_src).unwrap()),
            &args.include,
            args.verbose,
        ) {
            Ok(includes) => {
                for included in includes.iter() {
                    let relative_included_file = &included
                        .strip_prefix(&abs_src)
                        .unwrap_or(&included)
                        .to_path_buf();
                    included_files
                        .entry(relative_included_file.clone())
                        .or_insert_with(HashSet::new)
                        .insert(file.strip_prefix(&abs_src).unwrap_or(&file).to_path_buf());
                    if args.verbose {
                        let watch_str = if args.watch {
                            " and will be regenerated after any changes"
                        } else {
                            ""
                        };
                        println!(
                            "The file {:?} includes {:?} {:?}",
                            file, relative_included_file, watch_str
                        );
                    }
                }
            }
            Err(_e) => {}
        }
    }
    if !args.watch {
        return Ok(());
    }
    if args.verbose {
        println!("Watching for changes in {:?}, writing to {:?}", src, target);
    }
    let (tx, rx) = mpsc::channel::<Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;

    watcher.watch(Path::new(&abs_src), RecursiveMode::Recursive)?;

    // Block forever, handling events as they come in
    for res in rx {
        match res {
            Ok(event) => {
                if event.kind.is_access() {
                    continue;
                }
                if event.kind.is_remove() {
                    event.paths.iter().for_each(|path| {
                        let path = normalize_path(path);

                        let target_file = target.join(path.strip_prefix(&abs_src).unwrap());
                        if target_file.exists()
                            && target_file.is_file()
                            && target_file.starts_with(&target)
                        {
                            std::fs::remove_file(target_file.clone()).expect(
                                format!(
                                    "Failed to remove file {:?} when {:?} was removed",
                                    target_file, path
                                )
                                .as_str(),
                            );
                        }
                        if args.verbose {
                            println!(
                                "File removed: {:?}, removing target file: {:?}",
                                path, target_file
                            );
                        }
                    });
                    continue;
                } else {
                    event.paths.iter().for_each(|path| {
                    let path = normalize_path(path);
                    if args.verbose {
                        println!("File changed: {:?}, src: {:?}, change kind:{:?}", path, abs_src, event.kind);
                    }
                    if !path.starts_with(&abs_target) {
                        let file = path.clone();
                        let canon_file = canonicalize(file.clone()).unwrap_or(file.clone());
                        let relative_file = canon_file.strip_prefix(abs_src.clone());
                        if relative_file.is_err() {
                            if args.verbose {
                                eprintln!("{:?}{:?}{:?}", abs_src.clone(), file, relative_file.err());
                            }
                        } else {
                            let relative_file = relative_file.unwrap();
                            let target_file = target.join(relative_file);

                            match process_file(&file.clone(), &target_file, &args.include, args.verbose) {
                                Ok(includes) => {
                                    for included in includes.iter() {
                                        let relative_include = included.strip_prefix(abs_src.clone());
                                        if relative_include.is_err() {
                                            if args.verbose {
                                                eprintln!("{:?}{:?}{:?}", src, included, relative_include.err());
                                            }
                                        } else {
                                            included_files
                                                .entry(relative_include.unwrap().to_path_buf())
                                                .or_insert_with(HashSet::new)
                                                .insert(relative_file.to_path_buf());
                                        }
                                    }
                                }
                                Err(e) => {
                                    if args.verbose {
                                        println!("Error processing file {:?}: {:?}", file, e);
                                    }
                                }
                            };
                        }
                        let changed_file = &file.strip_prefix(&abs_src).unwrap_or(&file).to_path_buf();
                        if let Some(included) = included_files.get(changed_file) {
                            for included_file in included.iter() {
                                match process_file(
                                    &src.join(included_file),
                                    &target.join(included_file),
                                    &args.include,
                                    args.verbose,
                                ) {
                                    Ok(_includes) => {
                                        //the file we processed here has not changed so the includes have not changed
                                    },
                                    Err(e) => {
                                        match e.kind() {
                                            io::ErrorKind::NotFound => {
                                                if args.verbose {
                                                    println!("The file {:?} was included in {:?}, but was not found", included_file, file);
                                                }
                                            },
                                            io::ErrorKind::InvalidData => {
                                                if args.verbose {
                                                    println!("The file {:?} was included in {:?}, but contains binary data", included_file, file);
                                                }
                                            }
                                            _ => {
                                                println!("Error processing file {:?}. Error details: {:?}", included_file, e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                });
                }
            }
            Err(e) => println!("Error watching for changes. Error details: {:?}", e),
        }
    }

    Ok(())
}
pub fn are_paths_equal(path1: &Path, path2: &Path) -> bool {
    let norm_path1 = normalize_path(path1);
    let norm_path2 = normalize_path(path2);

    norm_path1 == norm_path2
}

pub fn list_of_paths(dir: &Path, target: &Path) -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| !are_paths_equal(e.path(), target))
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.into_path();
            paths.push(path);
        }
    }
    Ok(paths)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut result = PathBuf::new();

    while let Some(component) = components.next() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            _ => {
                result.push(component.as_os_str());
            }
        }
    }

    result
}

pub fn process_file(
    path: &Path,
    out_path: &Path,
    include_string: &str,
    verbose: bool,
) -> io::Result<Vec<PathBuf>> {
    let file = File::open(path);
    if file.is_err() {
        let e = file.err().unwrap();
        if e.kind() == io::ErrorKind::NotFound {
            if verbose {
                eprintln!("File not found: {:?}, skipping. If this looks like a temp file, it was probably deleted before we could parse and copy it.", path);
            }
        } else {
            eprintln!("Error opening file for processing: {:?}, {:?}. ", path, e);
        }
        return Err(e);
    }

    let reader = io::BufReader::new(file?);

    let mut new_content = String::new();
    let parent_dir = path.parent().unwrap_or_else(|| Path::new(""));
    let mut paths = Vec::new();
    for line in reader.lines() {
        match line {
            Ok(line) => {
                if line.starts_with(include_string) {
                    let include_path = line.trim_start_matches(include_string).trim();
                    let include_path = parent_dir.join(include_path);
                    let include_content = fs::read_to_string(include_path.clone());
                    match include_content {
                        Ok(include_content) => {
                            new_content.push_str(&include_content);
                        }
                        Err(e) => {
                            if verbose {
                                match e.kind() {
                                    io::ErrorKind::InvalidData => {
                                        println!(
                                            "Binary data in include file: {:?}, skipping",
                                            include_path
                                        );
                                    }
                                    io::ErrorKind::NotFound => {
                                        println!("Include file not found: {:?} (included in file {:?}), skipping", include_path, path);
                                    }
                                    _ => {
                                        println!(
                                            "Error reading include file: \"{:?}\" (included in file {:?}). Error: \"{:?}\", skipping",
                                            include_path,path, e
                                        );
                                    }
                                }
                            }
                            new_content.push_str(&line);
                        }
                    }

                    paths.push(normalize_path(&include_path));
                } else {
                    new_content.push_str(&line);
                };
            }
            Err(e) => {
                if verbose {
                    match e.kind() {
                        io::ErrorKind::InvalidData => {
                            println!("Binary data in file: {:?}, copying to {:?}", path, out_path);
                            std::fs::copy(path, out_path)?;
                            return Ok(Vec::new());
                        }
                        io::ErrorKind::NotFound => {
                            println!("File not found: {:?}, skipping", path);
                        }
                        _ => {
                            println!(
                                "Error reading file: \"{:?}\". Error: \"{:?}\", skipping",
                                path, e
                            );
                        }
                    }
                }
                return Err(e);
            }
        }
        new_content.push('\n');
    }
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(out_path)?;
    file.write_all(new_content.as_bytes())?;
    if verbose && !paths.is_empty() {
        println!("Input {:?}, Output {:?}", path, out_path);
    }
    Ok(paths)
}
