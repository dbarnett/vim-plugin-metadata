# Rust vim-plugin-metadata

Parse and analyze your vim plugins, from Rust!

WARNING: This library is early alpha, still missing tons of functionality, and probably has serious
bugs. Use at your own risk.

## Usage

`cargo add` it to your project, point it at a file, get metadata:

```rust
use vim_plugin_metadata::VimParser;

fn main() {
    let mut parser = VimParser::new().unwrap();
    let plugin = parser.parse_plugin_dir(".vim/plugged/someplugin").unwrap();
    println!("{plugin:#?}");
}
```
```
VimPlugin {
    content: [
        VimPluginSection {
            name: "plugin",
            nodes: [
                StandaloneDocComment( 
                    "Standalone header comment",
                ),
            ],
        },
        VimPluginSection {
            name: "autoload",
            nodes: [
                Function {
                    name: "someplugin#DoThing",
                    doc: "Does something cool.",
                },
            ],
        },
    ],
}
```

```rust
const VIMSCRIPT_CODE: &str = r#"
""
" Standalone header comment

""
" Does something cool.
func MyFunc() abort
  ...
endfunc
"#;

let module = parser.parse_module(VIMSCRIPT_CODE).unwrap();
println!("{module:#?}");
```
```
[
    StandaloneDocComment(
        "Standalone header comment",
    ),
    Function {
        name: "MyFunc",
        doc: Some(
            "Does something cool.",
        ),
    },
]
```

See tests in src/lib.rs for more usage examples.
