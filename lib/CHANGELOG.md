# Changelog (rust)

Changelog for https://crates.io/crates/vim-plugin-metadata.

## [Unreleased]
Major changes to parse_plugin_dir/parse_module_* signature and functionality

See https://github.com/dbarnett/vim-plugin-metadata/compare/v0.2.1...main.

Added:
- Parse more `VimNode` types: `Command` and `Flag`

Changed:
- Forked parse_module into parse_module_file and parse_module_str
- Change parse_module_* return type `Vec<VimNode>`->`VimModule`

## [0.2.x] - 2024-09-01
Adds parse_plugin_dir

### [0.2.1] - 2024-09-01

Fixes:
- Parsing now includes nodes it was randomly missing (#15)

### [0.2.0] - 2024-09-01

Added:
- Add `VimParser::parse_plugin_dir`

Changed:
- Change `VimParser::parse_module` to return `Vec<VimNode>`, get rid of VimModule

## [0.1.x] - 2024-08-30
Minimal initial version with basic parse_module

### [0.1.0] -2024-08-30
Minimal initial version

Added:
- `VimParser::parse_module` that can output `VimNode::Function` and `VimNode::StandaloneDocComment`
