 # A simple include preprocessor

 Reads all text files in the source directory and writes them to the target directory,
 replacing all instances of the include prefix followed by a file name with the contents of the included file.
 All subdirectories are also parsed, and copied to the target directory with the same structure.

 Can be set to watch for changes in the source directory and regenerate the files in the target directory. This
 is useful for development, where you want to see the changes in the target directory as you make them in the source directory.

 ## Example

 If we have the following files:
 File `main.rs`

 Contents:
 ```text
 --include disclaimer.txt
 struct Food {
 ...
 ```

 File `disclaimer.txt`

 Contents:
 ```text
 //Don't sue me if it breaks
 ```
 running simple-include would generate a file `target/main.rs` with the contents:
 ```text
 //Don't sue me if it breaks
 struct Food {
 ...
 ```

 If the -w (or --watch) value is set to true, the program stays running and will regenerate the
 target file if either main.rs or disclaimer.txt is changed.

 ## Include syntax

 The include prefix defaults to `--include` and can be set to something else with the -i (or --include) flag, but is always followed by a space then the filename.
 Paths can be relative (e.g. `../includes/header.txt`) or absolute (e.g. `/etc/motd`)

 Binary files will not be parsed, but will be copied to the target directory. This allows a typical use case where you want to run
 against a src folder and have all of the results copied to the target folder

 Do not use when you can't trust the src directory as it will include any file referenced in an include, even
 if it is outside of the src directory, so `--include /etc/passwd` would work if the program has the right permissions, for example.

 ## Usage

 A simple tool to include files in other files. Looks for lines with a given prefix and replaces them
with the contents of the file they point to. Can watch for changes in the source directory and keep the target directory in sync.
```
Usage: simple-include [OPTIONS]

Options:
  -w, --watch              Watch for changes in the source directory
  -s, --src <SRC>          Source directory [default: .]
  -t, --target <TARGET>    Target directory [default: target]
  -i, --include <INCLUDE>  Include Prefix [default: --include]
  -v, --verbose            Verbose output - prints the input and output file paths
  -h, --help               Print help
  -V, --version            Print version
```

 ## Status

 It works for me... The test cases cover only basic functionality. I have tested it on linux and windows for simple use cases.
 If you find a bug, please run with the -v flag and (if possible) let me know what the input and output files are, and I will try to fix it.
 If you are comfortable with rust (or even if you are not, but would like to try), feel free to submit a PR.
