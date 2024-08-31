use core::fmt;
use std::{error, str};

use tree_sitter::{LanguageError, Node, Parser, Point, TreeCursor};

#[derive(Debug, PartialEq)]
pub struct VimModule {
    pub nodes: Vec<VimNode>,
}

#[derive(Debug, PartialEq)]
pub enum VimNode {
    StandaloneDocComment(String),
    Function { name: String, doc: Option<String> },
}

#[derive(Default)]
pub struct VimParser {
    parser: Parser,
}

#[derive(Debug)]
pub enum Error {
    GrammarError(LanguageError),
    ParsingFailure,
}

impl From<LanguageError> for Error {
    fn from(e: LanguageError) -> Self {
        Self::GrammarError(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GrammarError(e) => write!(f, "Error loading grammar: {e}"),
            Self::ParsingFailure => {
                write!(f, "General failure from tree-sitter while parsing syntax")
            }
        }
    }
}

impl error::Error for Error {}

type Result<T> = core::result::Result<T, Error>;

impl VimParser {
    pub fn new() -> Result<VimParser> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_vim::language())?;
        Ok(VimParser { parser })
    }

    pub fn parse_module(&mut self, code: &str) -> Result<VimModule> {
        let tree = self.parser.parse(code, None).ok_or(Error::ParsingFailure)?;
        let mut tree_cursor = tree.walk();
        let mut nodes: Vec<VimNode> = Vec::new();
        let mut last_block_comment: Option<(String, Point)> = None;
        tree_cursor.goto_first_child();
        loop {
            let node = tree_cursor.node();
            if let Some((finished_comment_text, _)) =
                last_block_comment.take_if(|(_, next_pos)| *next_pos != node.start_position())
            {
                // Block comment wasn't immediately above the next node.
                // Treat it as bare standalone doc comment.
                nodes.push(VimNode::StandaloneDocComment(finished_comment_text));
            }
            match node.kind() {
                "comment" => {
                    if let Some((finished_comment_text, _)) = last_block_comment.take() {
                        // New comment block after dangling comment block.
                        nodes.push(VimNode::StandaloneDocComment(finished_comment_text));
                    }
                    last_block_comment =
                        VimParser::consume_block_comment(&mut tree_cursor, code.as_bytes());
                }
                "function_definition" => {
                    let doc = last_block_comment
                        .take()
                        .map(|(comment_text, _)| comment_text);
                    if let Some(funcname) =
                        VimParser::get_funcname_for_def(&mut tree_cursor, code.as_bytes())
                    {
                        nodes.push(VimNode::Function {
                            name: funcname.to_owned(),
                            doc,
                        });
                    } else {
                        eprintln!(
                            "Failed to find function name for function_definition at {:?}",
                            tree_cursor.node().start_position()
                        );
                    }
                }
                _ => {
                    // Silently ignore other node kinds.
                }
            }
            if !tree_cursor.goto_next_sibling() {
                break;
            }
        }
        // Consume any dangling last_block_comment.
        if let Some((comment_text, _)) = last_block_comment.take() {
            nodes.push(VimNode::StandaloneDocComment(comment_text));
        };
        Ok(VimModule { nodes })
    }

    fn get_funcname_for_def<'a>(tree_cursor: &mut TreeCursor, source: &'a [u8]) -> Option<&'a str> {
        let node = tree_cursor.node();
        assert_eq!(node.kind(), "function_definition");
        let decl = node
            .children(tree_cursor)
            .find(|c| c.kind() == "function_declaration");
        let ident = decl.and_then(|decl| {
            decl.children(tree_cursor)
                .find(|c| c.kind() == "identifier" || c.kind() == "scoped_identifier")
        });

        ident.as_ref().map(|n| VimParser::get_node_text(n, source))
    }

    fn consume_block_comment(
        tree_cursor: &mut TreeCursor,
        source: &[u8],
    ) -> Option<(String, Point)> {
        let node = tree_cursor.node();
        assert_eq!(node.kind(), "comment");
        let cur_pos = node.start_position();
        let mut next_pos = Point {
            row: cur_pos.row + 1,
            ..cur_pos
        };

        let mut comment_lines: Vec<String> = Vec::new();
        let comment_node_text = VimParser::get_node_text(&node, source);
        if let Some(leader_text) = comment_node_text.strip_prefix("\"\"") {
            // Valid leader, start comment block.
            if !leader_text.trim().is_empty() {
                // Treat trailing text after leader as first comment line.
                comment_lines.push(
                    leader_text
                        .strip_prefix(" ")
                        .unwrap_or(leader_text)
                        .to_owned(),
                );
            }
        } else {
            // Regular non-doc comment, ignore and let parsing skip.
            return None;
        }

        // Consume remaining comment lines at same indentation.
        while tree_cursor.goto_next_sibling() {
            let node = tree_cursor.node();
            if node.kind() != "comment" || node.start_position() != next_pos {
                // Back up so cursor still points to last consumed node.
                tree_cursor.goto_previous_sibling();
                break;
            }
            next_pos = Point {
                row: next_pos.row + 1,
                ..next_pos
            };
            let node_text = VimParser::get_node_text(&node, source);
            let comment_body = match &node_text[1..] {
                t if t.starts_with(" ") => &t[1..],
                t => t,
            };
            comment_lines.push(comment_body.to_owned());
        }
        Some((comment_lines.join("\n"), next_pos))
    }

    fn get_node_text<'a>(node: &Node, source: &'a [u8]) -> &'a str {
        str::from_utf8(&source[node.byte_range()]).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn parse_empty() {
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module("").unwrap();
        assert_eq!(module, VimModule { nodes: vec![] });
    }

    #[test]
    fn parse_one_nondoc_comment() {
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module("\" A comment").unwrap();
        assert_eq!(module, VimModule { nodes: vec![] });
    }

    #[test]
    fn parse_one_doc() {
        let code = r#"
""
" Foo
"#;
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module(code).unwrap();
        assert_eq!(
            module,
            VimModule {
                nodes: vec![VimNode::StandaloneDocComment("Foo".into())],
            }
        );
    }

    #[test]
    fn parse_messy_multiline_doc() {
        let code = r#"
"" Foo
"bar
"#;
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module(code).unwrap();
        assert_eq!(
            module,
            VimModule {
                nodes: vec![VimNode::StandaloneDocComment("Foo\nbar".into())],
            }
        );
    }

    #[test]
    fn parse_bare_function() {
        let code = r#"
func MyFunc() abort
  return 1
endfunc
"#;
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module(code).unwrap();
        assert_eq!(
            module,
            VimModule {
                nodes: vec![VimNode::Function {
                    name: "MyFunc".into(),
                    doc: None,
                }],
            }
        );
    }

    #[test]
    fn parse_doc_and_function() {
        let code = r#"
""
" Does a thing.
"
" Call and enjoy.
func MyFunc() abort
  return 1
endfunc
"#;
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module(code).unwrap();
        assert_eq!(
            module,
            VimModule {
                nodes: vec![VimNode::Function {
                    name: "MyFunc".into(),
                    doc: Some("Does a thing.\n\nCall and enjoy.".into()),
                }],
            }
        );
    }

    #[test]
    fn parse_two_docs() {
        let code = r#"
"" One doc

"" Another doc
"#;
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module(code).unwrap();
        assert_eq!(
            module,
            VimModule {
                nodes: vec![
                    VimNode::StandaloneDocComment("One doc".into()),
                    VimNode::StandaloneDocComment("Another doc".into()),
                ],
            }
        );
    }

    #[test]
    fn parse_different_doc_indentations() {
        let code = r#"
"" One doc
 " Ignored comment
"#;
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module(code).unwrap();
        assert_eq!(
            module,
            VimModule {
                nodes: vec![
                    VimNode::StandaloneDocComment("One doc".into()),
                    // Comment at different indentation is treated as a normal
                    // non-doc comment and ignored.
                ],
            }
        );
    }

    #[test]
    fn parse_autoload_funcname() {
        let code = "func foo#bar#Baz() | endfunc";
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module(code).unwrap();
        assert_eq!(
            module,
            VimModule {
                nodes: vec![VimNode::Function {
                    name: "foo#bar#Baz".into(),
                    doc: None
                },],
            }
        );
    }

    #[test]
    fn parse_scriptlocal_funcname() {
        let code = "func s:SomeFunc() | endfunc";
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module(code).unwrap();
        assert_eq!(
            module,
            VimModule {
                nodes: vec![VimNode::Function {
                    name: "s:SomeFunc".into(),
                    doc: None
                },],
            }
        );
    }

    #[test]
    fn parse_nested_func() {
        let code = r#"
function! Outer() abort
  let l:thing = {}
  function l:thing.Inner() abort
    return 1
  endfunction
  return l:thing
endfunction
"#;
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module(code).unwrap();
        assert_eq!(
            module,
            VimModule {
                nodes: vec![
                    VimNode::Function {
                        name: "Outer".into(),
                        doc: None
                    },
                    // TODO: Should have more nodes for inner function.
                ],
            }
        );
    }

    #[test]
    fn parse_unicode() {
        let code = r#"
""
" Fun stuff 🎈 ( ͡° ͜ʖ ͡°)
"#;
        let mut parser = VimParser::new().unwrap();
        let module = parser.parse_module(code).unwrap();
        assert_eq!(
            module,
            VimModule {
                nodes: vec![VimNode::StandaloneDocComment(
                    "Fun stuff 🎈 ( ͡° ͜ʖ ͡°)".into()
                )],
            }
        );
    }
}
