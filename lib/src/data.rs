#[derive(Debug, PartialEq)]
pub enum VimNode {
    StandaloneDocComment(String),
    Function { name: String, doc: Option<String> },
}

#[derive(Debug, PartialEq)]
pub struct VimPluginSection {
    pub name: String,
    pub nodes: Vec<VimNode>,
}

#[derive(Debug, PartialEq)]
pub struct VimPlugin {
    pub content: Vec<VimPluginSection>,
}
