//! `lower-scroll-unbounded` (LLP 1017.000 P1c): a `scroll` is refused only
//! when nothing the compiler can read bounds its height. A computed value is
//! taken as a bound and left to the bake's measured `bake-scroll-unbounded`.

use crate::values::{flex_basis_bounds, flex_longhands, numeric_literal};
use crate::{err, LowerError};
use contract_syntax::{Attr, Expr, Span};
use exact_kernel::StyleId;

/// Refuses a `scroll` that nothing bounds and whose parent stacks it, so it
/// would grow with its content and never scroll.
pub(crate) fn check(
    attrs: &[Attr],
    parent_stacks: bool,
    parent_bounded_column: bool,
    span: Span,
) -> Result<(), LowerError> {
    let has = |name: &str| attrs.iter().any(|a| a.name == name);
    let clips_y = attrs
        .iter()
        .any(|a| a.name == "overflow-y" && matches!(&a.value, Expr::Str(v, _) if v == "hidden"));
    if clips_y
        || !parent_stacks
        || has("height")
        || has("max-height")
        || flex_bounds(attrs)
        || insets_bound(attrs)
        || aspect_bounds(attrs)
        || shrinking_scroll(attrs, parent_bounded_column, true)
    {
        return Ok(());
    }
    err(
        "lower-scroll-unbounded",
        "`scroll` has no `height`, `max-height`, or `flex`, and its parent stacks it top to bottom, so it will grow with its content and never scroll",
        span,
    )
}

/// Whether a box lays its in-flow children out top to bottom, as far as its
/// literal `display` and `flex-direction` say. A flex row and a grid size
/// their items from the container (CSS Flexbox 1 §9.4 `stretch`, CSS Grid 1
/// §11.3), a `canvas` positions its own, and a computed `display` or
/// direction is the bake's.
pub(crate) fn stacks(tag: &str, fixed: &[(StyleId, &str)], attrs: &[Attr]) -> bool {
    if tag == "canvas" {
        return false;
    }
    let word = |name: &str, id, css: &str| match attrs.iter().rev().find(|a| a.name == name) {
        Some(Attr {
            value: Expr::Str(s, _),
            ..
        }) => Some(s.trim().to_ascii_lowercase()),
        Some(_) => None,
        None => Some(
            fixed
                .iter()
                .find(|(row, _)| *row == id)
                .map_or(css, |(_, v)| *v)
                .to_string(),
        ),
    };
    match word("display", StyleId::Display, "block").as_deref() {
        None | Some("grid" | "inline-grid") => false,
        Some("flex" | "inline-flex") => word("flex-direction", StyleId::FlexDirection, "row")
            .is_some_and(|d| d.starts_with("column")),
        Some(_) => true,
    }
}

/// Whether the flex grow and basis, from the `flex` shorthand and the
/// `flex-grow` and `flex-basis` longhands in the order written, bound the
/// box: a positive grow or a definite basis (`values::flex_basis_bounds`).
fn flex_bounds(attrs: &[Attr]) -> bool {
    let (mut grow, mut basis) = (Some(0.0), Some("auto".to_string()));
    for a in attrs {
        match a.name.as_str() {
            "flex" => {
                let parts = flex_longhands(&a.value);
                grow = parts.as_ref().map(|p| p.0);
                basis = parts.map(|p| p.2);
            }
            "flex-grow" => grow = numeric_literal(&a.value),
            "flex-basis" => {
                basis = match &a.value {
                    Expr::Str(s, _) => Some(s.clone()),
                    v => numeric_literal(v).map(|n| format!("{n}px")),
                }
            }
            _ => {}
        }
    }
    grow.is_none_or(|g| g > 0.0) || basis.is_none_or(|b| flex_basis_bounds(&b))
}

/// An absolutely positioned box is as tall as its vertical insets leave it
/// (CSS 2 §10.6.4), as `lower-zero-size` counts them.
fn insets_bound(attrs: &[Attr]) -> bool {
    let has = |name: &str| attrs.iter().any(|a| a.name == name);
    let positioned = attrs.iter().rev().find(|a| a.name == "position").is_some_and(|a| {
        !matches!(&a.value, Expr::Str(s, _) if matches!(s.as_str(), "static" | "relative" | "sticky"))
    });
    positioned && (has("inset") || has("top") && has("bottom"))
}

/// A width with an `aspect-ratio` gives the box its height (CSS Box Sizing 4
/// §5.1).
fn aspect_bounds(attrs: &[Attr]) -> bool {
    let set = |name: &str| {
        attrs.iter().rev().find(|a| a.name == name).is_some_and(
            |a| !matches!(&a.value, Expr::Str(s, _) if s.trim().eq_ignore_ascii_case("auto")),
        )
    };
    set("aspect-ratio") && set("width")
}

/// A shrinking flex item with a zero minimum fits a bounded flex column.
/// Its shrink is the `flex-shrink` longhand's, or else the `flex`
/// shorthand's (`flex="none"` does not shrink). A scroll container's
/// automatic minimum is zero (CSS Flexbox 1 §4.5), so for one (`scroller`)
/// an unwritten `min-height` is zero too.
pub(crate) fn shrinking_scroll(attrs: &[Attr], bounded_column: bool, scroller: bool) -> bool {
    let shrink = attrs
        .iter()
        .rev()
        .find(|a| a.name == "flex-shrink")
        .map(|a| numeric_literal(&a.value))
        .or_else(|| {
            let flex = attrs.iter().rev().find(|a| a.name == "flex")?;
            Some(flex_longhands(&flex.value).map(|(_, shrink, _)| shrink))
        });
    let min_height = attrs.iter().rev().find(|a| a.name == "min-height");
    bounded_column
        && min_height.map_or(scroller, |a| numeric_literal(&a.value) == Some(0.0))
        && shrink.is_none_or(|n| n.is_none_or(|n| n > 0.0))
}

pub(crate) fn bounded_column(tag: &str, attrs: &[Attr], inherited: bool) -> bool {
    let word = |name: &str| {
        attrs
            .iter()
            .rev()
            .find(|a| a.name == name)
            .and_then(|a| match &a.value {
                Expr::Str(s, _) => Some(s.as_str()),
                _ => None,
            })
    };
    let column = word("display").map_or(tag == "column" || tag == "button", |v| v == "flex")
        && word("flex-direction").map_or(tag == "column" || tag == "button", |v| {
            v.starts_with("column")
        });
    column
        && (attrs.iter().any(|a| {
            matches!(a.name.as_str(), "height" | "max-height")
                && !matches!(&a.value, Expr::Str(s, _) if s == "auto")
        }) || shrinking_scroll(attrs, inherited, tag == "scroll"))
}
