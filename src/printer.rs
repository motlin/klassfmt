//! CST -> `Doc` IR -> text.
//!
//! The style target is the canonical corpus. Most of the layout is a
//! deterministic vertical stack (one member per line, brace on its own line),
//! so the printer leans on hardlines and explicit indentation. The `pretty`
//! crate's `group`/`nest` machinery is reserved for the constructs that
//! actually reflow to fit the print width: criteria expressions, `orderBy`
//! lists, and long parameter/argument lists.
//!
//! Colon alignment (padding member names so their `:` line up within a block)
//! is a corpus hallmark that a Wadler printer cannot express directly; it is
//! computed during lowering by measuring sibling members and emitting padded
//! text literals.

use pretty::RcDoc;
use tree_sitter::Node;

mod comments;

use comments::CommentMap;

const INDENT: isize = 4;

/// Renders the whole compilation unit to a formatted string.
pub fn print(root: Node, source: &str, width: usize) -> String {
    let comments = CommentMap::new(root, source);
    let printer = Printer { source, comments };
    let doc = printer.compilation_unit(root);
    let mut out = String::new();
    doc.render_fmt(width, &mut out).expect("render to String");
    // Guarantee exactly one trailing newline.
    let trimmed = out.trim_end_matches('\n');
    format!("{trimmed}\n")
}

struct Printer<'a> {
    source: &'a str,
    comments: CommentMap,
}

type Doc<'a> = RcDoc<'a, ()>;

impl<'a> Printer<'a> {
    /// The verbatim source text of a node.
    fn text(&self, node: Node) -> &'a str {
        node.utf8_text(self.source.as_bytes()).unwrap()
    }

    /// Named children of `node`, skipping comment/extra nodes.
    fn named_children(&self, node: Node<'a>) -> Vec<Node<'a>> {
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .filter(|c| !is_comment(*c))
            .collect()
    }

    /// The first named child whose kind is one of `kinds`.
    fn child_of_kind(&self, node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
        self.named_children(node)
            .into_iter()
            .find(|c| kinds.contains(&c.kind()))
    }

    // ---- top level ----

    fn compilation_unit(&self, node: Node<'a>) -> Doc<'a> {
        let children = self.named_children(node);
        let mut parts: Vec<Doc<'a>> = Vec::new();

        for child in children {
            let doc = match child.kind() {
                "package_declaration" => self.package_declaration(child),
                "top_level_declaration" => self.top_level_declaration(child),
                other => self.verbatim_fallback(child, other),
            };
            parts.push(doc);
        }

        // One blank line between top-level items.
        intersperse_blank(parts)
    }

    fn package_declaration(&self, node: Node<'a>) -> Doc<'a> {
        let name = self.child_of_kind(node, &["package_name"]).unwrap();
        RcDoc::text("package ").append(RcDoc::text(self.text(name).to_string()))
    }

    fn top_level_declaration(&self, node: Node<'a>) -> Doc<'a> {
        let inner = self.named_children(node).into_iter().next().unwrap();
        match inner.kind() {
            "class_declaration" => self.class_declaration(inner),
            other => self.verbatim_fallback(inner, other),
        }
    }

    // ---- class ----

    fn class_declaration(&self, node: Node<'a>) -> Doc<'a> {
        let header = self.child_of_kind(node, &["class_header"]).unwrap();
        let block = self.child_of_kind(node, &["class_block"]).unwrap();
        self.class_header(header)
            .append(RcDoc::hardline())
            .append(self.class_block(block))
    }

    fn class_header(&self, node: Node<'a>) -> Doc<'a> {
        // classOrUser identifier abstract? extends? implements? serviceMods* classifierMods*
        let children = self.named_children(node);
        let mut inline: Vec<Doc<'a>> = Vec::new();
        let mut modifier_lines: Vec<Doc<'a>> = Vec::new();

        for child in children {
            match child.kind() {
                "class_or_user" => inline.push(RcDoc::text(self.text(child).to_string())),
                "identifier" => inline.push(RcDoc::text(self.text(child).to_string())),
                "abstract_declaration" => inline.push(RcDoc::text("abstract")),
                "extends_declaration" => inline.push(self.extends_declaration(child)),
                "implements_declaration" => inline.push(self.implements_declaration(child)),
                "classifier_modifier" | "class_service_modifier" => {
                    modifier_lines.push(RcDoc::text(self.text(child).to_string()));
                }
                other => inline.push(self.verbatim_fallback(child, other)),
            }
        }

        // "class Foo" (+ inline extends/implements), then each classifier
        // modifier on its own line indented one level. The modifier lines are
        // nested together so the indent applies once, not cumulatively.
        let head = spaced(inline);
        if modifier_lines.is_empty() {
            return head;
        }
        let mut tail = RcDoc::nil();
        for m in modifier_lines {
            tail = tail.append(RcDoc::hardline()).append(m);
        }
        head.append(tail.nest(INDENT))
    }

    fn extends_declaration(&self, node: Node<'a>) -> Doc<'a> {
        let r = self.child_of_kind(node, &["class_reference"]).unwrap();
        RcDoc::text("extends ").append(RcDoc::text(self.text(r).to_string()))
    }

    fn implements_declaration(&self, node: Node<'a>) -> Doc<'a> {
        let refs: Vec<String> = self
            .named_children(node)
            .into_iter()
            .filter(|c| c.kind() == "interface_reference")
            .map(|c| self.text(c).to_string())
            .collect();
        RcDoc::text("implements ").append(RcDoc::text(refs.join(", ")))
    }

    fn class_block(&self, node: Node<'a>) -> Doc<'a> {
        let members = self.named_children(node);
        self.member_block(&members, |p, m| p.class_member(m))
    }

    fn class_member(&self, node: Node<'a>) -> MemberDoc<'a> {
        // class_member wraps data_type_property | parameterized_property | association_end_signature
        let inner = self.named_children(node).into_iter().next().unwrap();
        match inner.kind() {
            "data_type_property" => self.data_type_property(inner),
            other => MemberDoc::unaligned(self.verbatim_fallback(inner, other)),
        }
    }

    // ---- data type properties ----

    fn data_type_property(&self, node: Node<'a>) -> MemberDoc<'a> {
        // data_type_property wraps primitive_property | enumeration_property
        let inner = self.named_children(node).into_iter().next().unwrap();
        self.property(inner)
    }

    /// Handles both primitive_property and enumeration_property: they share the
    /// shape `identifier ':' type optional? modifier* validation* ';'`.
    fn property(&self, node: Node<'a>) -> MemberDoc<'a> {
        let children = self.named_children(node);
        let name = self.text(children[0]);

        // Everything after the name and its ':' — type, optional marker,
        // modifiers, validations — joined with single spaces.
        let mut rhs: Vec<Doc<'a>> = Vec::new();
        for child in &children[1..] {
            match child.kind() {
                "primitive_type" | "enumeration_reference" => {
                    rhs.push(RcDoc::text(self.text(*child).to_string()))
                }
                "optional_marker" => {
                    // '?' attaches to the type with no leading space.
                    if let Some(last) = rhs.pop() {
                        rhs.push(last.append(RcDoc::text("?")));
                    }
                }
                "data_type_property_modifier" => rhs.push(RcDoc::text(self.text(*child).to_string())),
                "data_type_property_validation" => rhs.push(self.validation(*child)),
                other => rhs.push(self.verbatim_fallback(*child, other)),
            }
        }

        MemberDoc::aligned(name.to_string(), spaced(rhs))
    }

    fn validation(&self, node: Node<'a>) -> Doc<'a> {
        // e.g. `minLength(1)` — keyword directly followed by (literal).
        let inner = self.named_children(node).into_iter().next().unwrap();
        let children = self.named_children(inner);
        let keyword = self.text(children[0]);
        let param = self
            .child_of_kind(inner, &["integer_validation_parameter"])
            .unwrap();
        let literal = self.named_children(param).into_iter().next().unwrap();
        RcDoc::text(keyword.to_string())
            .append(RcDoc::text("("))
            .append(RcDoc::text(self.text(literal).to_string()))
            .append(RcDoc::text(")"))
    }

    // ---- shared block machinery ----

    /// Renders a `{ ... }` block of members with the brace on its own line,
    /// one member per line indented by `INDENT`, applying colon alignment
    /// across the members that opt into it.
    fn member_block<F>(&self, members: &[Node<'a>], render: F) -> Doc<'a>
    where
        F: Fn(&Self, Node<'a>) -> MemberDoc<'a>,
    {
        if members.is_empty() {
            return RcDoc::text("{").append(RcDoc::hardline()).append(RcDoc::text("}"));
        }

        let rendered: Vec<MemberDoc<'a>> = members.iter().map(|m| render(self, *m)).collect();

        // Alignment column: the widest name among aligned members.
        let align_width = rendered
            .iter()
            .filter_map(|m| m.align_name.as_ref().map(|n| n.chars().count()))
            .max()
            .unwrap_or(0);

        let mut body = RcDoc::nil();
        for (i, m) in rendered.into_iter().enumerate() {
            if i > 0 {
                body = body.append(RcDoc::hardline());
            }
            body = body.append(m.into_doc(align_width));
        }

        RcDoc::text("{")
            .append(RcDoc::hardline().append(body).nest(INDENT))
            .append(RcDoc::hardline())
            .append(RcDoc::text("}"))
    }

    // ---- fallback ----

    /// Emits a node's source text verbatim. Used for node kinds not yet handled
    /// by a dedicated printer method during incremental development.
    fn verbatim_fallback(&self, node: Node<'a>, _kind: &str) -> Doc<'a> {
        RcDoc::text(self.text(node).to_string())
    }
}

/// A rendered block member, optionally carrying its name so the enclosing block
/// can align colons. `align_name` is `Some(name)` for members that participate
/// in colon alignment (class/projection members) and `None` otherwise.
struct MemberDoc<'a> {
    align_name: Option<String>,
    /// The part after the aligned name and its `:` (for aligned members), or the
    /// whole member (for unaligned ones).
    rest: Doc<'a>,
    /// Trailing punctuation appended after `rest` (`;` or `,`), if any.
    terminator: &'static str,
}

impl<'a> MemberDoc<'a> {
    fn aligned(name: String, rest: Doc<'a>) -> Self {
        MemberDoc { align_name: Some(name), rest, terminator: ";" }
    }

    fn unaligned(doc: Doc<'a>) -> Self {
        MemberDoc { align_name: None, rest: doc, terminator: "" }
    }

    fn into_doc(self, align_width: usize) -> Doc<'a> {
        match self.align_name {
            Some(name) => {
                let pad = align_width.saturating_sub(name.chars().count());
                let padding: String = " ".repeat(pad);
                RcDoc::text(name)
                    .append(RcDoc::text(padding))
                    .append(RcDoc::text(": "))
                    .append(self.rest)
                    .append(RcDoc::text(self.terminator))
            }
            None => self.rest,
        }
    }
}

/// Joins docs with single spaces.
fn spaced<'a>(docs: Vec<Doc<'a>>) -> Doc<'a> {
    let mut iter = docs.into_iter();
    let mut acc = iter.next().unwrap_or_else(RcDoc::nil);
    for d in iter {
        acc = acc.append(RcDoc::text(" ")).append(d);
    }
    acc
}

/// Joins docs with a blank line (two hardlines) between each.
fn intersperse_blank<'a>(docs: Vec<Doc<'a>>) -> Doc<'a> {
    let mut iter = docs.into_iter();
    let mut acc = iter.next().unwrap_or_else(RcDoc::nil);
    for d in iter {
        acc = acc
            .append(RcDoc::hardline())
            .append(RcDoc::hardline())
            .append(d);
    }
    acc
}

fn is_comment(node: Node) -> bool {
    matches!(node.kind(), "line_comment" | "block_comment")
}
