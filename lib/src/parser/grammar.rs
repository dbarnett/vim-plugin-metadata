use std::collections::HashSet;
use std::iter::FusedIterator;
use std::str;
use std::sync::OnceLock;
use tree_sitter::{Language, Node, TreeCursor};
use tree_sitter_traversal::Cursor;

pub fn used_kinds() -> &'static HashSet<&'static str> {
    static USED_KINDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    USED_KINDS.get_or_init(|| {
        maplit::hashset! {
            "identifier",
            "string_literal",
            "list",
            "parameters",
            "spread",
            "call_expression",
            "command_attribute",
            "let",
            "list_assignment",
            "=",
        }
    })
}

pub fn vim_language() -> &'static Language {
    static LANGUAGE: OnceLock<Language> = OnceLock::new();
    LANGUAGE.get_or_init(tree_sitter_vim::language)
}

/// Thin convenience wrapper around Node.
#[derive(Clone, Debug)]
pub struct TreeNode<'tree, 'src> {
    pub treenode: Node<'tree>,
    source: &'src [u8],
}

impl<'tree, 'src> TreeNode<'tree, 'src> {
    pub fn is_kind(&self, kind: &str) -> bool {
        assert!(
            used_kinds().contains(kind),
            "Attempt to use kind not listed in grammar::used_kinds(): {kind}"
        );
        self.treenode.kind() == kind
    }

    pub fn get_text(&self) -> &'src str {
        str::from_utf8(&self.source[self.treenode.byte_range()]).unwrap()
    }

    pub fn children<'cursor>(
        &'cursor self,
        cursor: &'cursor mut TreeCursor<'tree>,
    ) -> impl ExactSizeIterator<Item = TreeNode<'tree, 'src>> + Sized + 'cursor {
        self.treenode
            .children(cursor)
            .map(move |c| Self::from((c, self.source)))
    }

    pub fn named_children<'cursor>(
        &'cursor self,
        cursor: &'cursor mut TreeCursor<'tree>,
    ) -> impl ExactSizeIterator<Item = TreeNode<'tree, 'src>> + Sized + 'cursor {
        self.treenode
            .named_children(cursor)
            .map(move |c| Self::from((c, self.source)))
    }

    pub fn named_child(&self, i: usize) -> Option<TreeNode<'tree, 'src>> {
        self.treenode
            .named_child(i)
            .map(|c| Self::from((c, self.source)))
    }

    pub fn child_by_field_name(&self, field_name: &str) -> Option<TreeNode<'tree, 'src>> {
        self.treenode
            .child_by_field_name(field_name)
            .map(|c| Self::from((c, self.source)))
    }

    pub fn next_named_sibling(&self) -> Option<TreeNode<'tree, 'src>> {
        self.treenode
            .next_named_sibling()
            .map(|c| Self::from((c, self.source)))
    }

    fn traverse_descendent_treenodes<C: Cursor>(
        &self,
        cursor: C,
    ) -> impl FusedIterator<Item = C::Node> + Sized {
        tree_sitter_traversal::traverse(cursor, tree_sitter_traversal::Order::Pre)
    }

    pub fn traverse_descendents<'cursor>(
        &'cursor self,
        cursor: &'cursor mut TreeCursor<'tree>,
    ) -> impl FusedIterator<Item = Self> + Sized + 'cursor {
        self.traverse_descendent_treenodes(cursor)
            .map(move |c| Self::from((c, self.source)))
    }
}

impl<'tree, 'src> From<(Node<'tree>, &'src [u8])> for TreeNode<'tree, 'src> {
    fn from((treenode, source): (Node<'tree>, &'src [u8])) -> Self {
        Self { treenode, source }
    }
}

impl<'tree, 'src> From<TreeNode<'tree, 'src>> for Node<'tree> {
    fn from(treenode: TreeNode<'tree, 'src>) -> Self {
        treenode.treenode
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::{Parser, Tree};

    #[test]
    fn used_kinds_defined() {
        let lang = vim_language();
        let mut missing_kinds = used_kinds().clone();
        for kind_id in 0..lang.node_kind_count() {
            missing_kinds.remove(lang.node_kind_for_id(kind_id as u16).unwrap());
        }
        assert!(
            missing_kinds.is_empty(),
            "Not all used kinds exist in tree_sitter_vim language: {missing_kinds:?}"
        );
    }

    #[test]
    fn treenode_get_text_empty() {
        let code = "";
        let tree = tree_from_code(code);
        assert_eq!(
            TreeNode::from((tree.root_node(), code.as_bytes())).get_text(),
            "",
        );
    }

    fn tree_from_code(code: &str) -> Tree {
        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_vim::language()).unwrap();
        parser.parse(code, None).unwrap()
    }
}
