#[derive(Debug, PartialEq)]
pub enum VimNode {
    StandaloneDocComment(String),
    Function { name: String, doc: Option<String> },
}
