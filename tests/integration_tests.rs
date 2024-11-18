use std::fs::{self, File};
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn test_process_file_with_includes() {
    let temp_dir = tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    let target_dir = temp_dir.path().join("target");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&target_dir).unwrap();

    let main_file_path = src_dir.join("main.txt");
    let include_file_path = src_dir.join("include.txt");

    let mut main_file = File::create(&main_file_path).unwrap();
    writeln!(main_file, "--include include.txt").unwrap();
    writeln!(main_file, "This is the main file.").unwrap();

    let mut include_file = File::create(&include_file_path).unwrap();
    writeln!(include_file, "This is the included file.").unwrap();

    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("--src")
        .arg(src_dir.to_str().unwrap())
        .arg("--target")
        .arg(target_dir.to_str().unwrap())
        .output()
        .expect("Failed to execute process");

    assert!(output.status.success());

    let output_content = fs::read_to_string(target_dir.join("main.txt")).unwrap();
    assert!(output_content.contains("This is the included file."));
    assert!(output_content.contains("This is the main file."));
}

#[test]
fn test_process_file_with_non_utf8_include() {
    let temp_dir = tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    let target_dir = temp_dir.path().join("target");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&target_dir).unwrap();

    let main_file_path = src_dir.join("main.txt");
    let include_file_path = src_dir.join("include.bin");

    let mut main_file = File::create(&main_file_path).unwrap();
    writeln!(main_file, "--include include.bin").unwrap();
    writeln!(main_file, "This is the main file.").unwrap();

    let mut include_file = File::create(&include_file_path).unwrap();
    include_file.write_all(&[0, 159, 146, 150]).unwrap(); // Some non-UTF-8 bytes

    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("--src")
        .arg(src_dir.to_str().unwrap())
        .arg("--target")
        .arg(target_dir.to_str().unwrap())
        .output()
        .expect("Failed to execute process");

    assert!(output.status.success());

    let output_content = fs::read_to_string(target_dir.join("main.txt")).unwrap();
    assert!(output_content.contains("--include include.bin"));
    assert!(!output_content.contains("This is the included file."));
}

#[test]
fn test_watch_functionality() {
    let temp_dir = tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    let target_dir = temp_dir.path().join("target");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&target_dir).unwrap();

    let main_file_path = src_dir.join("main.txt");
    println!("Main file path: {:?}", main_file_path);
    let include_file_path = src_dir.join("include.txt");
    println!("Include file path: {:?}", include_file_path);

    let mut main_file = File::create(&main_file_path).unwrap();
    writeln!(main_file, "--include include.txt").unwrap();
    writeln!(main_file, "This is the main file.").unwrap();
    main_file.flush().unwrap();

    let mut include_file = File::create(&include_file_path).unwrap();
    writeln!(include_file, "This is the included file.").unwrap();
    include_file.flush().unwrap();

    let mut child = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("--src")
        .arg(src_dir.to_str().unwrap())
        .arg("--target")
        .arg(target_dir.to_str().unwrap())
        .arg("--watch")
        .arg("-v")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start process");

    // Capture the stdout and stderr of the child process
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Spawn a thread to read the stdout
    let _stdout_handle = thread::spawn(move || {
        for line in stdout_reader.lines() {
            println!("stdout: {}", line.unwrap());
        }
    });

    // Spawn a thread to read the stderr
    let _stderr_handle = thread::spawn(move || {
        for line in stderr_reader.lines() {
            eprintln!("stderr: {}", line.unwrap());
        }
    });

    // Give the watcher some time to start
    thread::sleep(Duration::from_millis(100));

    // Modify the include file
    let mut include_file = File::create(&include_file_path).unwrap();
    writeln!(include_file, "This is the modified included file.").unwrap();

    // Give the watcher some time to detect the change and process the file
    thread::sleep(Duration::from_millis(100));

    let output_content = fs::read_to_string(target_dir.join("main.txt")).unwrap();
    println!("Output content: {}", output_content);
    assert!(output_content.contains("This is the modified included file."));
    assert!(output_content.contains("This is the main file."));

    // Kill the child process
    child.kill().expect("Failed to kill process");
}

#[test]
fn test_process_binary_file() {
    let temp_dir = tempdir().unwrap();
    let src_dir = temp_dir.path().join("src");
    let target_dir = temp_dir.path().join("target");

    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&target_dir).unwrap();

    let main_file_path = src_dir.join("main.txt");
    let binary_file_path = src_dir.join("binary.bin");

    let mut main_file = File::create(&main_file_path).unwrap();
    writeln!(main_file, "--include binary.bin").unwrap();
    writeln!(main_file, "This is the main file.").unwrap();

    let binary_content: Vec<u8> = vec![0, 159, 146, 150]; // Some non-UTF-8 bytes
    let mut binary_file = File::create(&binary_file_path).unwrap();
    binary_file.write_all(&binary_content).unwrap();
    binary_file.flush().unwrap();

    thread::sleep(Duration::from_millis(100));
    let output = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("-v")
        .arg("--src")
        .arg(src_dir.to_str().unwrap())
        .arg("--target")
        .arg(target_dir.to_str().unwrap())
        .output()
        .expect("Failed to execute process");

    thread::sleep(Duration::from_millis(100));
    assert!(output.status.success());
    output
        .stdout
        .iter()
        .for_each(|byte| print!("{}", *byte as char));
    let output_content = fs::read_to_string(target_dir.join("main.txt")).unwrap();
    assert!(output_content.contains("--include binary.bin"));
    assert!(output_content.contains("This is the main file."));

    let output_binary_content = fs::read(target_dir.join("binary.bin")).unwrap();
    assert_eq!(binary_content, output_binary_content);
}
