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
        VimModule {
            path: Some("plugin/somefile.vim"),
            nodes: [
                StandaloneDocComment( 
                    "Standalone header comment",
                ),
            ],
        },
        VimModule {
            path: Some("autoload/someplugin.vim"),
            nodes: [
                Function {
                    name: "someplugin#DoThing",
                    args: [],
                    modifiers: [],
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

let module = parser.parse_module_str(VIMSCRIPT_CODE).unwrap();
println!("{module:#?}");
```
```
VimModule {
    path: None,
    nodes: [
        StandaloneDocComment(
            "Standalone header comment",
        ),
        Function {
            name: "MyFunc",
            args: [],
            modifiers: [],
            doc: Some(
                "Does something cool.",
            ),
        },
    ]
}
```

See tests in src/lib.rs for more usage examples.
