# Changelog (py-bindings)

Changelog for https://pypi.org/project/vim-plugin-metadata.

Note the versioning loosely corresponds to versions for the [rust crate] dependency but isn't
identical, especially for patch versions.

[rust crate]: https://crates.io/crates/vim-plugin-metadata

## [1.0.x]
Major changes to parse_plugin_dir/parse_module_* signature and functionality

### [1.0.0-rc.0]
Added:
- Parse more `VimNode` types: `Command` and `Flag`

Changed:
- Fork parse_module into `parse_module_file` and `parse_module_str`
- Change parse_module_* return type `list[VimNode]`->`VimModule`
- Use pathlib types for returned paths and (optionally) path args

Fixed:
- Fix skipping after/ paths and some other subdirs like compiler/

## [0.2.x] - 2024-09-01
Adds parse_plugin_dir

### [0.2.2] - 2024-09-02

Added:
- py-bindings type annotations and some doc comments

### [0.2.1] - 2024-09-01
Bump rust dep to [0.2.1](https://github.com/dbarnett/vim-plugin-metadata/releases/tag/v0.2.1) (fixes
parsing not including all nodes).

### [0.2.0] - 2024-09-01

Added:
- Add VimParser.parse_plugin_dir

Changed:
- Change VimParser.parse_module to return `list[VimNode]`, get rid of VimModule

## [0.1.x] - 2024-08-30
Minimal initial version with basic parse_module

### [0.1.1] - 2024-08-31
Improved error handling

Added:
- Improved error handling
- Forked better README instead of reusing rust one

Changed:
- Simplified with top-level .parse_module

### [0.1.0] - 2024-08-30
Minimal initial version

Added:
- VimParser.parse_module that can output VimNode.Function and VimNode.StandaloneDocComment
