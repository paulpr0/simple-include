A simple tool to include files in other files. Looks for lines with a given prefix and replaces them
with the contents of the file they point to. Can watch for changes in the source directory and keep the target directory in sync.

Usage: simple-include [OPTIONS]

Options:
  -w, --watch              Watch for changes in the source directory
  -s, --src <SRC>          Source directory [default: .]
  -t, --target <TARGET>    Target directory [default: target]
  -i, --include <INCLUDE>  Include Prefix [default: --include]
  -v, --verbose            Verbose output - prints the input and output file paths
  -h, --help               Print help
  -V, --version            Print version
