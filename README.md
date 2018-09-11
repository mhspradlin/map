# Map
[![Build Status](https://travis-ci.org/mhspradlin/map.svg?branch=master)](https://travis-ci.org/mhspradlin/map)
[![Build status](https://ci.appveyor.com/api/projects/status/q34bb3qf43pgh2oo?svg=true)](https://ci.appveyor.com/project/mhspradlin/map)

A utility to map files into directories according to rules.

I created this to make it easier to organize similarly-named files into the same directory with human-friendly names. Specifically, this made it easier to organize all the book files from [Humble Bundles](https://www.humblebundle.com/) after [downloading them](https://gist.github.com/graymouser/a33fbb75f94f08af7e36) flat into my Downloads directory[1].

# Usage
Clone this repository, then build using `cargo build --release` using the stable toolchain.

## Options
* `-n, --dry-run` - If set, files and/or directories will not be created or deleted. This is useful to run with at least one level of verbosity to verify if this tool is doing what you expect.
* `-v[vv]` - Sets the level of verbosity. One `v` will output enough information to see when a file or directory will be created. Higher levels give you more information about rules and files being matched.
* `-r, --rules` - Specifies the file to be read for rules, which have a format of a single rule per line. See `examples` for what those look like. Exclusively specify this argument or a single rule as the first positional argument.
* `-s, --source-dir` - Specifies the directory to read for files to perform mappings on. Both currently supported rules do not recurse and only operate on regular files (i.e. not directories or symlinks).
* `-d, --dest-dir` - Specifies the directory to perform mappings into. For example, the relative destination specified in a Copy mapping is relative to this directory.

## Rules
Two rules are currently supported, for copying and moving files:
* Copy
  * Format: `c /<Regex>/ <Relative destination>`
  * Spaces before/after the `c` do not matter
  * Whitespace before/after the first non-whitespace characters of `<Relative destination>` are stripped
  * `<Relative destination>` may have multiple path components, all intermediate directories will be created
  * `<Regex>` is run against the file name (including extension) of each file in `source-dir`, not its entire path
  * Files that match the `<Regex>` are **copied** into `<dest-dir>/<Relative destination>/<Matched file name>`, preserving the original file
* Move
  * Format: `m /<Regex>/ <Relative destination>`
  * Spaces before/after the `m` do not matter
  * Whitespace before/after the first non-whitespace characters of `<Relative destination>` are stripped
  * `<Relative destination>` may have multiple path components, all intermediate directories will be created
  * `<Regex>` is run against the file name (including extension) of each file in `source-dir`, not its entire path
  * Files that match the `<Regex>` are **moved** into `<dest-dir>/<Relative destination>/<Matched file name>`, deleting the original file

## Examples
Dry-run a single rule to test moving files with `lime` in their name in `test-source` to `test-destination/Lime Files`:
```
map -s ./test-source -d ./test-destination -v -n 'm/lime/Lime Files'
```

Execute rules from a file `rules.map` on files in `test-source` with destination `test-destination`:
```
map -s ./test-source -d ./test-destination -r ./rules.map
```

## Errors
This tool attempts to catch errors before performing any filesystem modifications and for those that it doesn't it stops as soon as any errors are encountered. This tool first parses all the rules, determines what actions to perform (e.g. file moves and copies), then performs those actions. If there's a problem parsing the rules (e.g. invalid regex in a Copy rule) or determining the actions (e.g. the source directory cannot be read) then no filesystem modifications occur and a helpful (hopefully) error message is displayed.

If this tool encounters an error when performing actions (e.g. the destination directory is not writeable), then the tool stops performing actions immediately. It does not attempt to roll-back modifications that have already been made, so as always be careful with destructive filesystem actions like moving files with a Move action.

[1] It looks like that gist has since been improved to allow downloading into nicely-named folders.