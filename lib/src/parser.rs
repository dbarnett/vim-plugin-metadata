use crate::{Error, VimNode, VimPlugin, VimPluginSection};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::{fs, str};
use tree_sitter::{Node, Parser, Point, TreeCursor};
use walkdir::WalkDir;

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

    pub fn parse_plugin_dir<P: AsRef<Path> + Copy>(&mut self, path: P) -> crate::Result<VimPlugin> {
        let mut nodes_for_sections: HashMap<String, Vec<VimNode>> = HashMap::new();
        let section_order = ["instant", "plugin", "syntax", "autoload"];
        let sections_exclude = HashSet::from(["vroom"]);
        for entry in WalkDir::new(path) {
            let entry = entry?;
            if !(entry.file_type().is_file()
                && entry.file_name().to_string_lossy().ends_with(".vim"))
            {
                continue;
            }
            let section_name = entry
                .path()
                .strip_prefix(path)
                .unwrap()
                .iter()
                .nth(0)
                .expect("path should be a strict prefix of path under it")
                .to_string_lossy();
            if sections_exclude.contains(section_name.as_ref()) {
                continue;
            }
            let module_contents = fs::read_to_string(entry.path())?;
            let module_nodes = self.parse_module(module_contents.as_str())?;
            nodes_for_sections
                .entry(section_name.into())
                .or_default()
                .extend(module_nodes);
        }
        let sections = Self::sorted_by_partial_key_order(
            IntoIterator::into_iter(nodes_for_sections),
            &section_order,
        )
        .map(|(name, nodes)| VimPluginSection { name, nodes })
        .collect();
        Ok(VimPlugin { content: sections })
    }

    pub fn parse_module(&mut self, code: &str) -> crate::Result<Vec<VimNode>> {
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
                        Self::consume_block_comment(&mut tree_cursor, code.as_bytes());
                }
                "function_definition" => {
                    let doc = last_block_comment
                        .take()
                        .map(|(comment_text, _)| comment_text);
                    if let Some(funcname) =
                        Self::get_funcname_for_def(&mut tree_cursor, code.as_bytes())
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
        Ok(nodes)
    }

    fn sorted_by_partial_key_order<T>(
        iter: impl Iterator<Item = (String, Vec<T>)>,
        order: &[&str],
    ) -> impl Iterator<Item = (String, Vec<T>)> {
        let order_index: HashMap<_, _> = order
            .iter()
            .enumerate()
            .map(|(i, name)| (*name, i))
            .collect();
        iter.sorted_by(|(k1, _), (k2, _)| {
            Ord::cmp(
                &(order_index.get(k1.as_str()).unwrap_or(&order.len()), k1),
                &(order_index.get(k2.as_str()).unwrap_or(&order.len()), k2),
            )
        })
    }

    fn get_funcname_for_def<'a>(tree_cursor: &mut TreeCursor, source: &'a [u8]) -> Option<&'a str> {
        let node = tree_cursor.node();
        assert_eq!(node.kind(), "function_definition");
        let mut sub_cursor = node.walk();
        let decl = node
            .children(&mut sub_cursor)
            .find(|c| c.kind() == "function_declaration");
        let ident = decl.and_then(|decl| {
            decl.children(&mut sub_cursor)
                .find(|c| c.kind() == "identifier" || c.kind() == "scoped_identifier")
        });

        ident.as_ref().map(|n| Self::get_node_text(n, source))
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
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn parse_module_empty() {
        let mut parser = VimParser::new().unwrap();
        assert_eq!(parser.parse_module("").unwrap(), vec![]);
    }

    #[test]
    fn parse_module_one_nondoc_comment() {
        let mut parser = VimParser::new().unwrap();
        assert_eq!(parser.parse_module("\" A comment").unwrap(), vec![]);
    }

    #[test]
    fn parse_module_one_doc() {
        let code = r#"
""
" Foo
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module(code).unwrap(),
            vec![VimNode::StandaloneDocComment("Foo".into())]
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
            parser.parse_module(code).unwrap(),
            vec![VimNode::StandaloneDocComment("Foo\nbar".into())]
        );
    }

    #[test]
    fn parse_module_bare_function() {
        let code = r#"
func MyFunc() abort
  return 1
endfunc
"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module(code).unwrap(),
            vec![VimNode::Function {
                name: "MyFunc".into(),
                doc: None
            }]
        );
    }

    #[test]
    fn parse_module_doc_and_function() {
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
        assert_eq!(
            parser.parse_module(code).unwrap(),
            vec![VimNode::Function {
                name: "MyFunc".into(),
                doc: Some("Does a thing.\n\nCall and enjoy.".into()),
            }]
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
            parser.parse_module(code).unwrap(),
            vec![
                VimNode::StandaloneDocComment("One doc".into()),
                VimNode::StandaloneDocComment("Another doc".into()),
            ]
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
            parser.parse_module(code).unwrap(),
            vec![
                VimNode::StandaloneDocComment("One doc".into()),
                // Comment at different indentation is treated as a normal
                // non-doc comment and ignored.
            ]
        );
    }

    #[test]
    fn parse_module_two_funcs() {
        let code = r#"func FuncOne() | endfunc
func FuncTwo() | endfunc"#;
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module(code).unwrap(),
            vec![
                VimNode::Function {
                    name: "FuncOne".into(),
                    doc: None
                },
                VimNode::Function {
                    name: "FuncTwo".into(),
                    doc: None
                },
            ]
        );
    }

    #[test]
    fn parse_module_autoload_funcname() {
        let code = "func foo#bar#Baz() | endfunc";
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module(code).unwrap(),
            vec![VimNode::Function {
                name: "foo#bar#Baz".into(),
                doc: None
            }]
        );
    }

    #[test]
    fn parse_module_scriptlocal_funcname() {
        let code = "func s:SomeFunc() | endfunc";
        let mut parser = VimParser::new().unwrap();
        assert_eq!(
            parser.parse_module(code).unwrap(),
            vec![VimNode::Function {
                name: "s:SomeFunc".into(),
                doc: None
            }]
        );
    }

    #[test]
    fn parse_module_nested_func() {
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
        assert_eq!(
            parser.parse_module(code).unwrap(),
            vec![
                VimNode::Function {
                    name: "Outer".into(),
                    doc: None
                },
                // TODO: Should have more nodes for inner function.
            ]
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
            parser.parse_module(code).unwrap(),
            vec![VimNode::StandaloneDocComment(
                "Fun stuff 🎈 ( ͡° ͜ʖ ͡°)".into()
            )]
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
func! foo#Bar() abort
  sleep 1
endfunc
"#,
        );
        let plugin = parser.parse_plugin_dir(tmp_dir.path()).unwrap();
        assert_eq!(
            plugin,
            VimPlugin {
                content: vec![VimPluginSection {
                    name: "autoload".into(),
                    nodes: vec![VimNode::Function {
                        name: "foo#Bar".into(),
                        doc: None
                    }]
                }]
            }
        );
    }

    #[test]
    fn parse_plugin_dir_subdirs_instant_plugin_autoload_others() {
        let mut parser = VimParser::new().unwrap();
        let tmp_dir = tempdir().unwrap();
        create_plugin_file(tmp_dir.path(), "autoload/x.vim", "");
        create_plugin_file(tmp_dir.path(), "plugin/x.vim", "");
        create_plugin_file(tmp_dir.path(), "instant/x.vim", "");
        create_plugin_file(tmp_dir.path(), "other1/x.vim", "");
        create_plugin_file(tmp_dir.path(), "other2/x.vim", "");
        assert_eq!(
            parser.parse_plugin_dir(tmp_dir.path()).unwrap(),
            VimPlugin {
                content: vec![
                    VimPluginSection {
                        name: "instant".into(),
                        nodes: vec![],
                    },
                    VimPluginSection {
                        name: "plugin".into(),
                        nodes: vec![],
                    },
                    VimPluginSection {
                        name: "autoload".into(),
                        nodes: vec![],
                    },
                    VimPluginSection {
                        name: "other1".into(),
                        nodes: vec![],
                    },
                    VimPluginSection {
                        name: "other2".into(),
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
