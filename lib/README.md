# Rust vim-plugin-metadata

Parse and analyze your vim plugins, from Rust!

WARNING: This library is in early development, still missing functionality, and probably has plenty
of bugs. Use at your own risk.

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
            doc: Some("File header comment"),
            nodes: [],
        },
        VimModule {
            path: Some("autoload/someplugin.vim"),
            doc: None,
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
" File header comment

""
" Does something cool.
func MyFunc() abort
  …
endfunc
"#;

let module = parser.parse_module_str(VIMSCRIPT_CODE).unwrap();
println!("{module:#?}");
```
```
VimModule {
    path: None,
    doc: Some("File header comment"),
    nodes: [
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
