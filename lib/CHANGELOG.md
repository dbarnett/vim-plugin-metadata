# Changelog (rust)

Changelog for https://crates.io/crates/vim-plugin-metadata.

## [1.0.x]
Major changes to parse_plugin_dir/parse_module_* signature and functionality

### [1.0.0-rc.0] - 2024-09-09

Added:
- Parse more `VimNode` types: `Command` and `Flag`

Changed:
- Fork parse_module into parse_module_file and parse_module_str
- Change parse_module_* return type `Vec<VimNode>`->`VimModule`
- Tweak `VimNode::StandaloneDocComment` to have doc in a `doc` attribute for consistency, and add a
  general `VimNode::get_doc` getter

Fixed:
- Fix skipping after/ paths and some other subdirs like compiler/

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
