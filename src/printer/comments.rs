//! Comment/trivia attachment.
//!
//! tree-sitter keeps `line_comment` and `block_comment` nodes in the tree (they
//! are declared as `extras`), but they hang off arbitrary points rather than
//! being attached to the logical node they document. This module will, in A4,
//! attach each comment to a nearby node as leading / trailing / dangling
//! trivia so the printer can re-emit it in the right place.
//!
//! For A2/A3 it is a minimal placeholder: it simply records that no comments
//! have been attached yet. Fixtures used before A4 are comment-free.

use tree_sitter::Node;

#[derive(Default)]
pub struct CommentMap {
    // Populated in A4.
}

impl CommentMap {
    pub fn new(_root: Node, _source: &str) -> Self {
        CommentMap::default()
    }
}
