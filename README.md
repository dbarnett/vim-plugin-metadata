# Rust vim-plugin-metadata

Parse and analyze your vim plugins, from Rust!

WARNING: This library is early alpha, still missing tons of functionality, and probably has serious
bugs. Use at your own risk.

## Usage

`cargo add` it to your project, point it at a file, get metadata:

```rust
use vim_plugin_metadata::VimParser;

const VIMSCRIPT_CODE: &str = r#"
""
" Standalone header comment

""
" Does something cool.
func MyFunc() abort
  ...
endfunc
"#;

fn main() {
    let mut parser = VimParser::new();
    let module = parser.parse_module(VIMSCRIPT_CODE).unwrap();
    println!("{module:#?}");
}
```
```
VimModule {
    nodes: [
        StandaloneDocComment(
            "Standalone header comment",
        ),
        Function {
            name: "MyFunc",
            doc: Some(
                "Does something cool.",
            ),
        },
    ],
}
```

See tests in src/lib.rs for more usage examples.
