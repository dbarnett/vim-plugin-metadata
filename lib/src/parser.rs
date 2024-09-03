use crate::data::VimModule;
use crate::{Error, VimNode, VimPlugin};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::{fs, str};
use tree_sitter::{Node, Parser, Point, TreeCursor};
use walkdir::WalkDir;

// TODO: Also support "after" equivalents.
const DEFAULT_SECTION_ORDER: [&str; 9] = [
    "plugin", "instant", "autoload", "syntax", "indent", "ftdetect", "ftplugin", "spell", "colors",
];

#[derive(Default)]
pub struct VimParser {
    parser: Parser,
}

impl VimParser {
    pub fn new() -> crate::Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_vim::language())?;
        Ok(Self { parser })
    }

    /// Parses all supported metadata from a single plugin at the given path.
    pub fn parse_plugin_dir<P: AsRef<Path> + Copy>(&mut self, path: P) -> crate::Result<VimPlugin> {
        let mut modules_for_sections: HashMap<String, Vec<VimModule>> = HashMap::new();
        let sections_to_include = HashSet::from(DEFAULT_SECTION_ORDER);
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if !(entry.file_type().is_file()
                && entry.file_name().to_string_lossy().ends_with(".vim"))
            {
                continue;
            }
            let relative_path = entry.path().strip_prefix(path).unwrap();
            let section_name = relative_path
                .iter()
                .nth(0)
                .expect("path should be a strict prefix of path under it")
                .to_string_lossy();
            if !sections_to_include.contains(section_name.as_ref()) {
                continue;
            }
            let module = self.parse_module_file(entry.path())?;
            // Replace absolute path with one relative to plugin root.
            let module = VimModule {
                path: relative_path.to_owned().into(),
                ..module
            };
            modules_for_sections
                .entry(section_name.into())
                .or_default()
                .push(module);
        }
        let modules = DEFAULT_SECTION_ORDER
            .iter()
            .flat_map(|section_name| {
                modules_for_sections
                    .remove(*section_name)
                    .unwrap_or_default()
            })
            .collect();
        Ok(VimPlugin { content: modules })
    }

    /// Parses and returns metadata for a single module (a.k.a. file) of vimscript code.
    pub fn parse_module_file<P: AsRef<Path>>(&mut self, path: P) -> crate::Result<VimModule> {
        let code = fs::read_to_string(path.as_ref())?;
        let module = self.parse_module_str(&code)?;
        Ok(VimModule {
            path: Some(path.as_ref().to_owned()),
            ..module
        })
    }

    /// Parses and returns metadata for a single module (a.k.a. file) of vimscript code.
    pub fn parse_module_str(&mut self, code: &str) -> crate::Result<VimModule> {
        let tree = self.parser.parse(code, None).ok_or(Error::ParsingFailure)?;
        let mut tree_cursor = tree.walk();
        let mut doc = None;
        let mut nodes: Vec<VimNode> = Vec::new();
        let mut last_block_comment: Option<(String, Point)> = None;
        let mut reached_end = !tree_cursor.goto_first_child();
        while !reached_end {
            let node = tree_cursor.node();
            match node.kind() {
                "comment" => {
                    assert!(last_block_comment.is_none());
                    last_block_comment =
                        Self::consume_block_comment(&mut tree_cursor, code.as_bytes());
                }
                "function_definition" => {
                    let doc = last_block_comment
                        .take()
                        .map(|(comment_text, _)| comment_text);
                    let source = code.as_bytes();
                    match Self::new_function_from_node(&tree_cursor.node(), doc, source) {
                        Ok(function) => {
                            nodes.push(function);
                        }
                        Err(err) => eprintln!("{err}"),
                    }
                }
                _ => {
                    // Silently ignore other node kinds.
                }
            }
            reached_end = !tree_cursor.goto_next_sibling();
            if let Some((finished_comment_text, _)) = last_block_comment.take_if(|(_, next_pos)| {
                reached_end
                    || tree_cursor.node().kind() == "comment"
                    || *next_pos != tree_cursor.node().start_position()
            }) {
                // Block comment wasn't immediately above the next node.
                // Treat it as bare standalone doc comment.
                if doc.is_none() && nodes.is_empty() {
                    // This standalone doc comment is the first one in the module.
                    // Treat it as overall module doc.
                    doc = Some(finished_comment_text);
                } else {
                    nodes.push(VimNode::StandaloneDocComment(finished_comment_text));
                }
            }
        }
        Ok(VimModule {
            path: None,
            doc,
            nodes,
        })
    }

    fn new_function_from_node(
        node: &Node,
        doc: Option<String>,
        source: &[u8],
    ) -> Result<VimNode, String> {
        assert_eq!(node.kind(), "function_definition");
        let mut cursor = node.walk();

        let mut decl = None;
        let mut modifiers = vec![];
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function" | "endfunction" => {}
                "function_declaration" => {
                    decl = Some(child);
                }
                "body" => {
                    break;
                }
                // Everything else between function_declaration and body is a modifier.
                _ => {
                    modifiers.push(Self::get_node_text(&child, source).to_string());
                }
            }
        }
        let ident = decl.and_then(|decl| {
            decl.children(&mut cursor)
                .find(|c| c.kind() == "identifier" || c.kind() == "scoped_identifier")
        });
        let ident = match ident {
            Some(ident) => ident,
            None => {
                return Err(format!(
                    "Failed to find function name for function_definition at {:?}",
                    node.start_position()
                ));
            }
        };

        let params = decl.and_then(|decl| {
            decl.children(&mut cursor)
                .find(|c| c.kind() == "parameters")
        });

        Ok(VimNode::Function {
            name: Self::get_node_text(&ident, source).to_string(),
            args: params
                .map(|params| {
                    params
                        .children(&mut cursor)
                        .filter(|c| c.kind() == "identifier" || c.kind() == "spread")
                        .map(|c| Self::get_node_text(&c, source).to_string())
                        .collect()
                })
                .unwrap_or_default(),
            modifiers,
            doc,
        })
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
        let comment_node_text = Self::get_node_text(&node, source);
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
            let node_text = Self::get_node_text(&node, source);
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
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    #[test]
    fn parse_module_empty() {
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str("").unwrap(),
            VimModule {
                path: None,
                doc: None,
                nodes: vec![]
            }
        );
    }

    #[test]
    fn parse_module_one_nondoc_comment() {
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str("\" A comment").unwrap(),
            VimModule {
                path: None,
                doc: None,
                nodes: vec![]
            }
        );
    }

    #[test]
    fn parse_module_one_doc() {
        let code = r#"
""
" Foo
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: "Foo".to_string().into(),
                nodes: vec![]
            }
        );
    }

    #[test]
    fn parse_module_messy_multiline_doc() {
        let code = r#"
"" Foo
"bar
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: "Foo\nbar".to_string().into(),
                nodes: vec![]
            }
        );
    }

    #[test]
    fn parse_module_bare_function() {
        let code = r#"
func MyFunc()
  return 1
endfunc
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: None,
                nodes: vec![VimNode::Function {
                    name: "MyFunc".into(),
                    args: vec![],
                    modifiers: vec![],
                    doc: None
                }]
            }
        );
    }

    #[test]
    fn parse_module_doc_and_function() {
        let code = r#"
""
" Does a thing.
"
" Call and enjoy.
func MyFunc()
  return 1
endfunc
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: None,
                nodes: vec![VimNode::Function {
                    name: "MyFunc".into(),
                    args: vec![],
                    modifiers: vec![],
                    doc: Some("Does a thing.\n\nCall and enjoy.".into()),
                }]
            }
        );
    }

    #[test]
    fn parse_module_func_with_args() {
        let code = r#"
func MyFunc(arg1, arg2)
  return 1
endfunc
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: None,
                nodes: vec![VimNode::Function {
                    name: "MyFunc".into(),
                    args: vec!["arg1".into(), "arg2".into()],
                    modifiers: vec![],
                    doc: None
                }]
            }
        );
    }

    #[test]
    fn parse_module_func_with_opt_args_and_modifiers() {
        let code = r#"
func! MyFunc(arg1, ...) range dict abort
  return 1
endfunc
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: None,
                nodes: vec![VimNode::Function {
                    name: "MyFunc".into(),
                    args: vec!["arg1".into(), "...".into()],
                    modifiers: vec!["!".into(), "range".into(), "dict".into(), "abort".into()],
                    doc: None
                }]
            }
        );
    }

    #[test]
    fn parse_module_two_docs() {
        let code = r#"
"" One doc

"" Another doc
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: Some("One doc".into()),
                nodes: vec![VimNode::StandaloneDocComment("Another doc".into()),]
            }
        );
    }

    #[test]
    fn parse_module_different_doc_indentations() {
        let code = r#"
"" One doc
 " Ignored comment
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: Some("One doc".into()),
                nodes: vec![
                    // Comment at different indentation is treated as a normal
                    // non-doc comment and ignored.
                ],
            }
        );
    }

    #[test]
    fn parse_module_two_funcs() {
        let code = r#"func FuncOne() | endfunc
func FuncTwo() | endfunc"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: None,
                nodes: vec![
                    VimNode::Function {
                        name: "FuncOne".into(),
                        args: vec![],
                        modifiers: vec![],
                        doc: None
                    },
                    VimNode::Function {
                        name: "FuncTwo".into(),
                        args: vec![],
                        modifiers: vec![],
                        doc: None
                    },
                ]
            }
        );
    }

    #[test]
    fn parse_module_autoload_funcname() {
        let code = "func foo#bar#Baz() | endfunc";
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: None,
                nodes: vec![VimNode::Function {
                    name: "foo#bar#Baz".into(),
                    args: vec![],
                    modifiers: vec![],
                    doc: None
                }]
            }
        );
    }

    #[test]
    fn parse_module_scriptlocal_funcname() {
        let code = "func s:SomeFunc() | endfunc";
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: None,
                nodes: vec![VimNode::Function {
                    name: "s:SomeFunc".into(),
                    args: vec![],
                    modifiers: vec![],
                    doc: None
                }]
            }
        );
    }

    #[test]
    fn parse_module_nested_func() {
        let code = r#"
function Outer()
  let l:thing = {}
  function l:thing.Inner()
    return 1
  endfunction
  return l:thing
endfunction
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: None,
                nodes: vec![
                    VimNode::Function {
                        name: "Outer".into(),
                        args: vec![],
                        modifiers: vec![],
                        doc: None
                    },
                    // TODO: Should have more nodes for inner function.
                ]
            }
        );
    }

    #[test]
    fn parse_module_unicode() {
        let code = r#"
""
" Fun stuff 🎈 ( ͡° ͜ʖ ͡°)
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module_str(code).unwrap(),
            VimModule {
                path: None,
                doc: Some("Fun stuff 🎈 ( ͡° ͜ʖ ͡°)".into()),
                nodes: vec![],
            }
        );
    }

    #[test]
    fn parse_plugin_dir_empty() {
        let mut parser = VimParser::new().unwrap();
        let tmp_dir = tempdir().unwrap();
        let plugin = parser.parse_plugin_dir(tmp_dir.path()).unwrap();
        assert_eq!(plugin, VimPlugin { content: vec![] });
    }

    #[test]
    fn parse_plugin_dir_one_autoload_func() {
        let mut parser = VimParser::new().unwrap();
        let tmp_dir = tempdir().unwrap();
        create_plugin_file(
            tmp_dir.path(),
            "autoload/foo.vim",
            r#"
func foo#Bar()
  sleep 1
endfunc
"#,
        );
        let plugin = parser.parse_plugin_dir(tmp_dir.path()).unwrap();
        assert_eq!(
            plugin,
            VimPlugin {
                content: vec![VimModule {
                    path: PathBuf::from("autoload/foo.vim").into(),
                    doc: None,
                    nodes: vec![VimNode::Function {
                        name: "foo#Bar".into(),
                        args: vec![],
                        modifiers: vec![],
                        doc: None
                    }]
                }],
            }
        );
    }

    #[test]
    fn parse_plugin_dir_various_subdirs() {
        let mut parser = VimParser::new().unwrap();
        let tmp_dir = tempdir().unwrap();
        create_plugin_file(tmp_dir.path(), "ignored_not_in_subdir.vim", "");
        create_plugin_file(tmp_dir.path(), "autoload/x.vim", "");
        create_plugin_file(tmp_dir.path(), "instant/x.vim", "");
        create_plugin_file(tmp_dir.path(), "plugin/x.vim", "");
        create_plugin_file(tmp_dir.path(), "colors/x.vim", "");
        create_plugin_file(tmp_dir.path(), "spell/x.vim", "");
        assert_eq!(
            parser.parse_plugin_dir(tmp_dir.path()).unwrap(),
            VimPlugin {
                content: vec![
                    VimModule {
                        path: PathBuf::from("plugin/x.vim").into(),
                        doc: None,
                        nodes: vec![],
                    },
                    VimModule {
                        path: PathBuf::from("instant/x.vim").into(),
                        doc: None,
                        nodes: vec![],
                    },
                    VimModule {
                        path: PathBuf::from("autoload/x.vim").into(),
                        doc: None,
                        nodes: vec![],
                    },
                    VimModule {
                        path: PathBuf::from("spell/x.vim").into(),
                        doc: None,
                        nodes: vec![],
                    },
                    VimModule {
                        path: PathBuf::from("colors/x.vim").into(),
                        doc: None,
                        nodes: vec![],
                    },
                ]
            }
        );
    }

    fn create_plugin_file<P: AsRef<Path>>(root: &Path, subpath: P, contents: &str) {
        let filepath = root.join(subpath);
        fs::create_dir_all(filepath.parent().unwrap()).unwrap();
        fs::write(filepath, contents).unwrap()
    }
}
