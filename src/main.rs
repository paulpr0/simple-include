use notify::{Event, RecursiveMode, Result, Watcher};
use std::fs::{self, File};
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

    let src = Path::new(&args.src);
    let target = Path::new(&args.target);

    for file in list_of_paths(&src, &target)? {
        process_file(
            &file,
            &target.join(file.clone().strip_prefix(src).unwrap()),
            &args.include,
            args.verbose,
        )?;
    }
    if !args.watch {
        return Ok(());
    }
    if args.verbose {
        println!("Watching for changes in {:?}, writing to {:?}", src, target);
    }
    let (tx, rx) = mpsc::channel::<Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;

    watcher.watch(Path::new(src), RecursiveMode::Recursive)?;

    let abs_src = fs::canonicalize(Path::new(&args.src))?;
    let abs_target = fs::canonicalize(Path::new(&args.target))?;

    // Block forever, handling events as they come in
    for res in rx {
        match res {
            Ok(event) => {
                event.paths.iter().for_each(|path| {
                    if !path.starts_with(&abs_target) {
                        let file = Path::new(path);
                        process_file(
                            &file,
                            &target.join(file.strip_prefix(abs_src.clone()).unwrap()),
                            &args.include,
                            args.verbose,
                        )
                        .unwrap();
                    }
                });
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}
fn are_paths_equal(path1: &Path, path2: &Path) -> bool {
    let norm_path1 = normalize_path(path1);
    let norm_path2 = normalize_path(path2);

    norm_path1 == norm_path2
}

fn list_of_paths(dir: &Path, target: &Path) -> io::Result<Vec<PathBuf>> {
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

fn process_file(
    path: &Path,
    out_path: &Path,
    include_string: &str,
    verbose: bool,
) -> io::Result<()> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut new_content = String::new();
    let mut include_found = false;
    let parent_dir = path.parent().unwrap_or_else(|| Path::new(""));

    for line in reader.lines() {
        let line = line?;
        if line.starts_with(include_string) {
            let include_path = line.trim_start_matches(include_string).trim();
            let include_path = parent_dir.join(include_path);
            let include_content = fs::read_to_string(include_path)?;
            new_content.push_str(&include_content);
            include_found = true;
        } else {
            new_content.push_str(&line);
        }
        new_content.push('\n');
    }
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(out_path)?;
    file.write_all(new_content.as_bytes())?;
    if verbose && include_found {
        println!("Input {:?}, Output {:?}", path, out_path);
    }
    Ok(())
}
