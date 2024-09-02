/// A representation of a single high-level grammar token of vim syntax,
/// such as a comment or function.
#[derive(Debug, PartialEq)]
pub enum VimNode {
    StandaloneDocComment(String),
    Function { name: String, doc: Option<String> },
}

/// A section of a plugin, such as the "autoload" subdirectory.
#[derive(Debug, PartialEq)]
pub struct VimPluginSection {
    pub name: String,
    pub nodes: Vec<VimNode>,
}

/// An entire vim plugin with all the metadata parsed from its files.
#[derive(Debug, PartialEq)]
pub struct VimPlugin {
    pub content: Vec<VimPluginSection>,
}
