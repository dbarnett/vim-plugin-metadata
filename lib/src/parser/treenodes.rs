use crate::parser::grammar::TreeNode;
use crate::VimNode;
use std::fmt::Formatter;
use std::{fmt, str};
use unicode_ellipsis::truncate_str;

pub struct TreeNodeMetadata<'a> {
    pub treenodes: Vec<TreeNode<'a, 'a>>,
    pub doc: Option<String>,
}

impl fmt::Debug for TreeNodeMetadata<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut nodes_formatted = vec![];
        for node in self.treenodes.iter() {
            nodes_formatted.push(format!(
                "Node {{ kind: {:?}, range: {:?}, text: {:?} }}",
                node.treenode.kind(),
                node.treenode.range(),
                truncate_str(node.get_text(), 500),
            ));
        }
        f.debug_struct("TreeNodeMetadata")
            .field("treenodes", &nodes_formatted.join(", "))
            .field("doc", &self.doc)
            .finish()
    }
}

impl<'a> TreeNodeMetadata<'a> {
    fn try_get_treenode(&self) -> Result<&TreeNode<'a, 'a>, String> {
        match &self.treenodes[..] {
            [treenode] => Ok(treenode),
            _ => Err(format!(
                "Attempted to process single tree node but found multiple: {self:?}"
            )),
        }
    }

    pub(crate) fn kind(&self) -> &'a str {
        let kind = self.treenodes[0].treenode.kind();
        for treenode in &self.treenodes {
            if treenode.treenode.kind() != kind {
                panic!("Found different kinds for single node: {:?}", self);
            }
        }
        kind
    }

    fn get_func_node(&self) -> Result<VimNode, String> {
        let treenode = self.try_get_treenode()?;
        let mut cursor = treenode.treenode.walk();
        let mut decl: Option<TreeNode> = None;
        let mut modifiers = vec![];
        for child in treenode.children(&mut cursor) {
            match child.treenode.kind() {
                "function" | "endfunction" => {}
                "function_declaration" => {
                    decl = Some(child);
                }
                "body" => {
                    break;
                }
                // Everything else between function_declaration and body is a modifier.
                _ => {
                    modifiers.push(child.get_text().to_string());
                }
            }
        }
        let name = decl
            .clone()
            .and_then(|decl| decl.child_by_field_name("name"))
            .map(|ident| ident.get_text())
            .ok_or_else(|| {
                format!(
                    "Failed to find function name for {} at {:?}",
                    treenode.treenode.kind(),
                    treenode.treenode.start_position(),
                )
            })?;
        let params =
            decl.and_then(|decl| decl.children(&mut cursor).find(|c| c.is_kind("parameters")));
        let args: Vec<_> = params
            .map(|params| {
                params
                    .children(&mut cursor)
                    .filter(|c| c.is_kind("identifier") || c.is_kind("spread"))
                    .map(|c| c.get_text().to_string())
                    .collect()
            })
            .unwrap_or_default();
        Ok(VimNode::Function {
            name: name.to_string(),
            args,
            modifiers,
            doc: self.doc.clone(),
        })
    }

    fn get_command_node(&self) -> Result<VimNode, String> {
        let treenode = self.try_get_treenode()?;
        let name = treenode
            .child_by_field_name("name")
            .map(|n| n.get_text())
            .ok_or_else(|| {
                format!(
                    "Failed to find command name for {} at {:?}",
                    treenode.treenode.kind(),
                    treenode.treenode.start_position(),
                )
            })?;
        let mut cursor = treenode.treenode.walk();
        let modifiers: Vec<_> = treenode
            .children(&mut cursor)
            .filter(|c| c.is_kind("command_attribute"))
            .map(|c| c.get_text().to_string())
            .collect();
        Ok(VimNode::Command {
            name: name.to_string(),
            modifiers,
            doc: self.doc.clone(),
        })
    }

    fn get_flag_node(&self) -> Result<Option<VimNode>, String> {
        let treenode = self.try_get_treenode()?;
        let mut cursor = treenode.treenode.walk();
        let call_exp = treenode
            .children(&mut cursor)
            .find(|c| c.is_kind("call_expression"));
        if let Some(func_expr) = call_exp.and_then(|call| call.child_by_field_name("function")) {
            let mut cursor = func_expr.treenode.walk();
            let last_func_id = func_expr
                .traverse_descendents(&mut cursor)
                .filter(|n| n.is_kind("identifier"))
                .last();
            if last_func_id.is_some_and(|func_id| func_id.get_text() == "Flag") {
                let arg1 = func_expr.next_named_sibling();
                let arg2 = arg1.clone().and_then(|a1| a1.next_named_sibling());
                match arg1 {
                    Some(arg1) if arg1.is_kind("string_literal") => {
                        // Matched call Flag(arg1, arg2, ...).
                        let flag_name_literal = arg1.get_text();
                        let flag_name = if let Some(flag_name) = flag_name_literal
                            .strip_prefix("'")
                            .and_then(|l| l.strip_suffix("'"))
                        {
                            flag_name.to_string()
                        } else {
                            quoted_string::unquote_unchecked(flag_name_literal).into()
                        };
                        let default_value = arg2.map(|a2| a2.get_text().to_string());
                        return Ok(Some(VimNode::Flag {
                            name: flag_name,
                            default_value_token: default_value,
                            doc: self.doc.clone(),
                        }));
                    }
                    _ => {}
                }
            }
        }

        Ok(None)
    }

    pub(crate) fn maybe_consume_doc(&mut self, doc: &mut Option<TreeNodeMetadata>) {
        if !matches!(
            self.kind(),
            "function_definition" | "command_statement" | "call_statement" | "let_statement"
        ) {
            return;
        }
        if let Some(VimNode::StandaloneDocComment(consumed_doc)) = doc.take().and_then(|doc| {
            let mut doc_nodes: Vec<VimNode> = doc.into();
            // TODO: Use all nodes or error if multiple.
            doc_nodes.pop()
        }) {
            self.doc = Some(consumed_doc);
        }
    }
}

impl<'a> From<TreeNode<'a, 'a>> for TreeNodeMetadata<'a> {
    fn from(treenode: TreeNode<'a, 'a>) -> Self {
        Self {
            treenodes: vec![treenode],
            doc: None,
        }
    }
}

impl<'a> From<TreeNodeMetadata<'a>> for Vec<VimNode> {
    fn from(metadata: TreeNodeMetadata) -> Self {
        match metadata.kind() {
            "comment" => {
                let mut doc_lines = vec![];
                let first_node = &metadata.treenodes[0];
                let first_line = first_node.get_text();
                if let Some(leader_content) = first_line.strip_prefix("\"\"") {
                    // Valid leader, start comment block.
                    if !leader_content.trim().is_empty() {
                        // Treat trailing text after leader as first comment line.
                        doc_lines.push(leader_content.strip_prefix(" ").unwrap_or(leader_content));
                    }
                } else {
                    // Regular non-doc comment, ignore and let parsing skip.
                    return vec![];
                }
                for treenode in &metadata.treenodes[1..] {
                    let comment_text = treenode.get_text();
                    let comment_content = comment_text.strip_prefix("\"").unwrap();
                    doc_lines.push(comment_content.strip_prefix(" ").unwrap_or(comment_content));
                }
                vec![VimNode::StandaloneDocComment(
                    doc_lines.join("\n").trim_end().to_string(),
                )]
            }
            "function_definition" => {
                let mut nodes = vec![];
                match metadata.get_func_node() {
                    Ok(node) => {
                        nodes.push(node);
                    }
                    Err(err) => {
                        eprintln!("{err}");
                    }
                }
                nodes
            }
            "command_statement" => {
                let mut nodes = vec![];
                match metadata.get_command_node() {
                    Ok(node) => {
                        nodes.push(node);
                    }
                    Err(err) => {
                        eprintln!("{err}");
                    }
                }
                nodes
            }
            "let_statement" => metadata.try_get_treenode().map_or_else(
                |err| {
                    eprintln!("{err}");
                    vec![]
                },
                |treenode| {
                    let mut nodes = vec![];
                    // Extract identifier and its next named sibling from node like:
                    // (let_statement (identifier) SOME_RHS)
                    let mut cursor = treenode.treenode.walk();
                    let children = treenode.children(&mut cursor).collect::<Vec<_>>();
                    match &children[..] {
                        [cmd, _, op, _, ..] if !cmd.is_kind("let") || !op.is_kind("=") => {
                            // Ignore types of let_statement besides standard assignment.
                            // For example, let+= isn't defining a new variable.
                        }
                        [_, lhs, _, rhs, ..] if lhs.is_kind("list_assignment") => {
                            // Destructuring assignment.
                            let rhs_is_literal = rhs.is_kind("list")
                                && lhs.treenode.named_child_count()
                                    == rhs.treenode.named_child_count();
                            for (i, lhs) in lhs.named_children(&mut cursor).enumerate() {
                                let rhs_str = if rhs_is_literal {
                                    let rhs_item = rhs.named_child(i).unwrap();
                                    rhs_item.get_text().to_string()
                                } else {
                                    format!("{}[{}]", rhs.get_text(), i)
                                };
                                nodes.push(VimNode::Variable {
                                    name: lhs.get_text().to_string(),
                                    init_value_token: rhs_str,
                                    doc: metadata.doc.clone(),
                                });
                            }
                        }
                        [_, lhs, _, rhs, ..] => {
                            // Standard assignment.
                            nodes.push(VimNode::Variable {
                                name: lhs.get_text().to_string(),
                                init_value_token: rhs.get_text().to_string(),
                                doc: metadata.doc.clone(),
                            });
                        }
                        _ => {}
                    }

                    nodes
                },
            ),
            "call_statement" => match metadata.get_flag_node() {
                Ok(Some(flag_node)) => vec![flag_node],
                Ok(None) => vec![],
                Err(err) => {
                    eprintln!("{err}");
                    vec![]
                }
            },
            "ERROR" => {
                let first_err_node = &metadata.treenodes[0];
                let start_pos = first_err_node.treenode.start_position();
                eprintln!(
                    "Syntax error at ({}, {}) near {:?}",
                    start_pos.row,
                    start_pos.column,
                    first_err_node.get_text(),
                );
                vec![]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tree_sitter::{Parser, Tree};

    #[test]
    fn metadata_into_nodes_empty_func() {
        let code = "func SomeFunc() | endfunc";
        let tree = tree_from_code(code);
        let nodes: Vec<_> = node_metadata_from_code(&tree, code).into();
        assert_eq!(
            nodes,
            vec![VimNode::Function {
                name: "SomeFunc".into(),
                args: vec![],
                modifiers: vec![],
                doc: None,
            }]
        );
    }

    #[test]
    fn metadata_into_nodes_func_missing_name() {
        let code = "func () | endfunc";
        let tree = tree_from_code(code);
        let nodes: Vec<_> = node_metadata_from_code(&tree, code).into();
        assert_eq!(
            nodes,
            vec![
                // Function skipped (printed to stderr instead).
            ]
        );
    }

    #[test]
    fn metadata_into_nodes_command_missing_name() {
        let code = r"command -bang";
        let tree = tree_from_code(code);
        let nodes: Vec<_> = node_metadata_from_code(&tree, code).into();
        assert_eq!(
            nodes,
            vec![
                // Command skipped (printed to stderr instead).
            ]
        );
    }

    #[test]
    fn metadata_into_nodes_let_missing_rhs() {
        let code = r"let somevar";
        let tree = tree_from_code(code);
        let nodes: Vec<_> = node_metadata_from_code(&tree, code).into();
        assert_eq!(
            nodes,
            vec![
                // let_statement skipped (not an assignment).
            ]
        );
    }

    #[test]
    fn metadata_into_nodes_let_compound_assignment() {
        let code = r"let somevar += 1";
        let tree = tree_from_code(code);
        let nodes: Vec<_> = node_metadata_from_code(&tree, code).into();
        assert_eq!(
            nodes,
            vec![
                // let_statement skipped (compound assignment vs initial declaration).
            ]
        );
    }

    #[test]
    fn metadata_into_nodes_let_destructuring_assignment() {
        let code = r"let [var1, var2] = [1, 2]";
        let tree = tree_from_code(code);
        let mut metadata = node_metadata_from_code(&tree, code);
        set_doc(
            &mut metadata,
            r#"
""
" Some doc
"#,
        );
        let nodes: Vec<_> = metadata.into();
        assert_eq!(
            nodes,
            vec![
                VimNode::Variable {
                    name: "var1".to_string(),
                    init_value_token: "1".to_string(),
                    doc: Some("Some doc".into()),
                },
                VimNode::Variable {
                    name: "var2".to_string(),
                    init_value_token: "2".to_string(),
                    // Note: same doc attaches to all items.
                    doc: Some("Some doc".into()),
                },
            ]
        );
    }

    #[test]
    fn metadata_into_nodes_let_destructuring_rhs_nonliteral() {
        let code = r"let [var1, var2] = SomeFunc()";
        let tree = tree_from_code(code);
        let nodes: Vec<_> = node_metadata_from_code(&tree, code).into();
        assert_eq!(
            nodes,
            vec![
                VimNode::Variable {
                    name: "var1".to_string(),
                    init_value_token: "SomeFunc()[0]".to_string(),
                    doc: None,
                },
                VimNode::Variable {
                    name: "var2".to_string(),
                    init_value_token: "SomeFunc()[1]".to_string(),
                    doc: None,
                },
            ]
        );
    }

    fn set_doc(metadata: &mut TreeNodeMetadata, doc_code: &str) {
        let doc_tree = tree_from_code(doc_code);
        let mut cursor = doc_tree.walk();
        cursor.goto_first_child();
        let doc_treenode: TreeNode = (cursor.node(), doc_code.as_bytes()).into();
        let mut doc_metadata: TreeNodeMetadata = doc_treenode.into();
        while cursor.goto_next_sibling() {
            doc_metadata
                .treenodes
                .push(TreeNode::from((cursor.node(), doc_code.as_bytes())));
        }
        let mut doc_metadata = Some(doc_metadata);
        metadata.maybe_consume_doc(&mut doc_metadata);
        assert!(doc_metadata.is_none());
    }

    fn tree_from_code(code: &str) -> Tree {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_vim::language()).unwrap();
        parser.parse(code, None).unwrap()
    }

    fn node_metadata_from_code<'a>(tree: &'a Tree, code: &'a str) -> TreeNodeMetadata<'a> {
        let mut cursor = tree.walk();
        cursor.goto_first_child();
        TreeNode::from((cursor.node(), code.as_bytes())).into()
    }
}
