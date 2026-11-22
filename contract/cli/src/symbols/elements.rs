//! Authored elements for editor navigation and focused source lookups.

use super::{close, Resolver};
use crate::{sources::Sources, CompileError};
use contract_lower::tags;
use contract_syntax::*;
use exact_kernel::PropId;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::rc::Rc;

/// An authored view element: its tag and each attribute it was written with.
pub(super) struct Element {
    tag: String,
    span: Span,
    component: Option<String>,
    attrs: Vec<ElementAttr<String>>,
}
struct ElementAttr<T> {
    name: T,
    span: Span,
    /// A string literal value and where it is, quotes included.
    string: Option<(T, Span)>,
}

impl Resolver<'_> {
    pub(super) fn element(&mut self, tag: &str, span: Span, attrs: &[Attr]) {
        if !self.graph.editor {
            return;
        }
        // A button's one positional is the parser's `text` child, at the button's tag.
        if self.graph.elements.last().is_some_and(|e| e.span == span) {
            return;
        }
        self.graph.elements.push(Element {
            tag: tag.into(),
            span,
            component: self.component.map(|c| c.name.clone()),
            attrs: written_attrs(attrs, span)
                .map(|attr| ElementAttr::from(attr).into_owned())
                .collect(),
        });
    }
}

fn written_attrs(attrs: &[Attr], site: Span) -> impl Iterator<Item = &Attr> {
    // The parser derives heading attributes from the tag.
    attrs.iter().filter(move |a| a.written && a.span != site)
}

impl<'a> From<&'a Attr> for ElementAttr<&'a str> {
    fn from(a: &'a Attr) -> Self {
        Self {
            name: &a.name,
            span: a.span,
            string: match &a.value {
                Expr::Str(value, at) => Some((value, *at)),
                _ => None,
            },
        }
    }
}

impl ElementAttr<&str> {
    fn into_owned(self) -> ElementAttr<String> {
        ElementAttr {
            name: self.name.into(),
            span: self.span,
            string: self.string.map(|(value, span)| (value.into(), span)),
        }
    }
}

fn place(span: Span) -> Value {
    json!({ "col": span.col, "end_col": span.end_col, "line": span.line })
}

impl<T: AsRef<str>> ElementAttr<T> {
    fn json(&self) -> Value {
        let mut attr = place(self.span);
        attr["name"] = self.name.as_ref().into();
        if let Some((value, span)) = &self.string {
            attr["string"] = place(*span);
            attr["string"]["value"] = value.as_ref().into();
        }
        attr
    }
}

fn element_json(sources: &Sources, span: Span, tag: &str, component: Option<&str>) -> Value {
    let mut one = place(span);
    one["file"] = sources.path(span).to_string_lossy().into();
    one["tag"] = tag.into();
    if let Some(component) = component {
        one["component"] = component.into();
    }
    one
}

pub(super) fn write(out: &mut Vec<u8>, sources: &Sources, elements: &[Element]) {
    for (i, element) in elements.iter().enumerate() {
        if i != 0 {
            out.push(b',');
        }
        let mut one = element_json(
            sources,
            element.span,
            &element.tag,
            element.component.as_deref(),
        );
        one["attrs"] = Value::Array(element.attrs.iter().map(ElementAttr::json).collect());
        serde_json::to_writer(&mut *out, &one).expect("writing JSON into a Vec cannot fail");
    }
}

/// One group of filters for `symbols --editor`. Entries of `text` are alternatives.
#[derive(Default)]
pub struct ElementFilters<'a> {
    /// The declared component name.
    pub component: Option<&'a str>,
    /// The compiler tag or its authored spelling.
    pub tag: Option<&'a str>,
    /// Substrings of authored text and attributes.
    pub text: Vec<&'a str>,
    /// Attributes that must be present.
    pub has: Vec<&'a str>,
    /// Attributes whose authored values must match exactly.
    pub values: Vec<(&'a str, &'a str)>,
}

type Range = (Span, Span);

struct SourceText<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> SourceText<'a> {
    fn new(text: &'a str) -> Self {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(at, _)| at + 1))
            .collect();
        Self { text, starts }
    }

    fn slice(&self, (start, end): Range) -> Option<&'a str> {
        let a = self.starts.get(start.line.checked_sub(1)? as usize)?
            + start.col.checked_sub(1)? as usize;
        let b = self.starts.get(end.line.checked_sub(1)? as usize)?
            + end.end_col.checked_sub(1)? as usize;
        self.text.get(a..b)
    }
}

fn source_json((start, end): Range, source: &str) -> Value {
    let mut value = json!({
        "col": start.col, "end_col": end.end_col, "line": start.line, "source": source
    });
    if start.line != end.line {
        value["end_line"] = end.line.into();
    }
    value
}

fn positional_text<'a>(tag: &str, positional: &'a [Expr]) -> Option<&'a Expr> {
    let [text_argument, ..] = positional else {
        return None;
    };
    (tags::tag(tag).and_then(|tag| tag.positional) == Some(PropId::Text)).then_some(text_argument)
}

fn element_text_ranges<'a>(
    node: &'a Node,
    names: &'a NameSpans,
) -> impl Iterator<Item = Range> + 'a {
    let Node::Element {
        tag,
        positional,
        children,
        ..
    } = node
    else {
        unreachable!();
    };
    let own = positional_text(tag, positional);
    let child_positions = children.iter().filter_map(move |child| match child {
        Node::Element {
            tag: child_tag,
            positional,
            ..
        } if tag == "button" && child_tag == "text" => positional_text(child_tag, positional),
        _ => None,
    });
    own.into_iter()
        .chain(child_positions)
        .filter_map(move |expr| names.view_ranges.get(&expr.span()).copied())
}

struct Context {
    span: Span,
    branch: Option<&'static str>,
    outer: Option<Rc<Context>>,
}

impl Context {
    fn for_child_lists(node: &Node, outer: &Option<Rc<Self>>) -> [Option<Rc<Self>>; 2] {
        let nested = |span, branch| {
            Some(Rc::from(Self {
                span,
                branch,
                outer: outer.clone(),
            }))
        };
        match node {
            Node::When { span, .. } => [nested(*span, Some("then")), nested(*span, Some("else"))],
            Node::Each { span, .. } => [nested(*span, None), None],
            _ => [outer.clone(), outer.clone()],
        }
    }

    fn json(&self, text: &SourceText<'_>, names: &NameSpans) -> Option<Value> {
        let range = *names.view_ranges.get(&self.span)?;
        let mut row = source_json(range, text.slice(range)?);
        row["kind"] = if self.branch.is_some() { "if" } else { "for" }.into();
        if let Some(branch) = self.branch {
            row["branch"] = branch.into();
        }
        Some(row)
    }
}

struct ViewElement<'a> {
    node: &'a Node,
    tag: &'a str,
    span: Span,
    attrs: &'a [Attr],
    positional: &'a [Expr],
    context: Option<Rc<Context>>,
}

fn authored_elements(component: &Component) -> impl Iterator<Item = ViewElement<'_>> {
    let mut previous = None;
    Node::walk_with(&component.view, None, |authored_node, outer| {
        Context::for_child_lists(authored_node, outer).map(Some)
    })
    .filter_map(move |(node, context)| {
        let Node::Element {
            tag,
            span,
            attrs,
            positional,
            ..
        } = node
        else {
            return None;
        };
        // A positional button label shares the button's tag span.
        (previous.replace(*span) != Some(*span)).then_some(ViewElement {
            node,
            tag,
            span: *span,
            attrs,
            positional,
            context,
        })
    })
}

struct Site<'a> {
    node: &'a Node,
    tag: &'a str,
    span: Span,
    attrs: &'a [Attr],
    positional: &'a [Expr],
    part: Option<&'a anatomy::Mark>,
    text: &'a SourceText<'a>,
    names: &'a NameSpans,
}

impl Site<'_> {
    fn attributes(&self) -> impl Iterator<Item = (ElementAttr<&str>, Option<Range>)> {
        let part = self.part.map(|mark| ElementAttr {
            name: anatomy::PART,
            span: mark.span,
            string: mark.literal.map(|span| (mark.part.as_str(), span)),
        });
        let bare = self.positional.iter().filter_map(|expr| {
            let Expr::Ident(name, span) = expr else {
                return None;
            };
            (is_input_switch(self.tag, expr) || is_input_multiple(self.tag, expr)).then_some((
                ElementAttr {
                    name: name.as_str(),
                    span: *span,
                    string: None,
                },
                None,
            ))
        });
        written_attrs(self.attrs, self.span)
            .map(ElementAttr::from)
            .chain(part)
            .map(|attr| {
                let range = self.names.view_ranges.get(&attr.span).copied();
                (attr, range)
            })
            .chain(bare)
    }

    fn binding(&self, range: Option<Range>) -> Option<(Range, &str)> {
        let range = range?;
        Some((range, self.text.slice(range)?))
    }

    fn labels(&self) -> impl Iterator<Item = (Range, &str)> {
        element_text_ranges(self.node, self.names)
            .filter_map(|r| self.text.slice(r).map(|s| (r, s)))
    }

    fn matches(&self, component: &str, filters: &ElementFilters<'_>) -> bool {
        let authored_tag = self.text.slice((self.span, self.span)).unwrap_or(self.tag);
        filters.component.is_none_or(|want| want == component)
            && filters
                .tag
                .is_none_or(|want| want == self.tag || want == authored_tag)
            && filters
                .has
                .iter()
                .all(|name| self.attributes().any(|(a, _)| a.name == *name))
            && filters.values.iter().all(|(name, value)| {
                self.attributes().any(|(a, range)| {
                    a.name == *name
                        && self
                            .binding(range)
                            .is_some_and(|(_, source)| source == *value)
                })
            })
            && (filters.text.is_empty()
                || filters.text.iter().any(|needle| {
                    authored_tag.contains(needle)
                        || self.labels().any(|(_, s)| s.contains(needle))
                        || self.attributes().any(|(a, range)| {
                            a.name.contains(needle)
                                || self.binding(range).is_some_and(|(_, s)| s.contains(needle))
                        })
                }))
    }

    fn json(&self, sources: &Sources, component: &str, context: Option<&Context>) -> Value {
        let mut one = element_json(sources, self.span, self.tag, Some(component));
        let mut attrs: Vec<_> = self.attributes().collect();
        attrs.sort_by_key(|(attr, _)| attr.span);
        one["attrs"] = Value::Array(
            attrs
                .into_iter()
                .map(|(a, range)| {
                    let mut attr = a.json();
                    if let (None, Some((range, source))) = (a.string, self.binding(range)) {
                        attr["binding"] = source_json(range, source);
                    }
                    attr
                })
                .collect(),
        );
        let labels: Vec<_> = self.labels().map(|(r, s)| source_json(r, s)).collect();
        if !labels.is_empty() {
            one["text"] = labels.into();
        }
        if context.is_some() {
            let mut context: Vec<_> = std::iter::successors(context, |c| c.outer.as_deref())
                .filter_map(|c| c.json(self.text, self.names))
                .collect();
            context.reverse();
            one["context"] = context.into();
        }
        one
    }
}

/// Matches in one parsed source, ordered by the author's tag locations.
pub(super) fn lookup(sources: &Sources, filters: &[ElementFilters<'_>], file: &File) -> Vec<Value> {
    let Some(first) = file.components.first() else {
        return Vec::new();
    };
    let text = SourceText::new(&sources.texts[first.span.source_id as usize]);
    let mut matches = Vec::new();
    for c in &file.components {
        let parts: HashMap<_, _> = c
            .anatomy
            .marks
            .iter()
            .map(|mark| (mark.node, mark))
            .collect();
        for ViewElement {
            node,
            tag,
            span,
            attrs,
            positional,
            context,
        } in authored_elements(c)
        {
            let site = Site {
                node,
                tag,
                span,
                attrs,
                positional,
                part: parts.get(&span).copied(),
                text: &text,
                names: &file.names,
            };
            if filters.iter().any(|group| site.matches(&c.name, group)) {
                matches.push((span, site.json(sources, &c.name, context.as_deref())));
            }
        }
    }
    matches.sort_by_key(|(span, _)| (span.line, span.col));
    matches.into_iter().map(|(_, row)| row).collect()
}

pub(super) fn write_filtered(elements: &[Value], errors: &[CompileError]) -> String {
    let mut out = b"{\"elements\":".to_vec();
    serde_json::to_writer(&mut out, elements).expect("writing JSON into a Vec cannot fail");
    close(out, errors)
}
