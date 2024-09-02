use std::path::PathBuf;

/// A representation of a single high-level grammar token of vim syntax,
/// such as a comment or function.
#[derive(Debug, PartialEq)]
pub enum VimNode {
    StandaloneDocComment(String),
    Function {
        name: String,
        args: Vec<String>,
        modifiers: Vec<String>,
        doc: Option<String>,
    },
}

/// An individual module (a.k.a. file) of vimscript code.
#[derive(Debug, PartialEq)]
pub struct VimModule {
    pub path: PathBuf,
    pub nodes: Vec<VimNode>,
}

/// An entire vim plugin with all the metadata parsed from its files.
#[derive(Debug, PartialEq)]
pub struct VimPlugin {
    pub content: Vec<VimModule>,
}
