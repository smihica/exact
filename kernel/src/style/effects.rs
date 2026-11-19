//! `filter` and `clip-path` (LLP 1055.000 D10, D14; CSS Filter Effects and
//! CSS Masking): their grammars, each linked by use (LLP 1047 D2).
//!
//! An artifact links them when its plan binds the row (and `filter`'s when
//! it draws SVG, whose filters read it), and a plan that needs one unlinked
//! is refused at boot (D6), so an unlinked artifact never parses one. The
//! web links both with the `effects` capability; Apple draws `clip-path` in
//! its core and links that grammar always. [`crate::Kernel::new`] and the
//! compiler link both. The kernel's own tests parse directly.

use crate::clip::ClipPath;
use crate::svg::filter::FilterList;

/// A row's grammar: its value, or `None` when CSS refuses the text.
#[cfg(not(test))]
type Grammar<T> = fn(&str) -> Option<T>;

#[cfg(not(test))]
static FILTER: std::sync::OnceLock<Grammar<FilterList>> = std::sync::OnceLock::new();
#[cfg(not(test))]
static CLIP_PATH: std::sync::OnceLock<Grammar<ClipPath>> = std::sync::OnceLock::new();

/// Link both grammars: what the `effects` capability links.
pub fn link() {
    link_filter();
    link_clip_path();
}

/// Link `filter`'s grammar, which SVG's filters read too.
pub fn link_filter() {
    #[cfg(not(test))]
    let _ = FILTER.set(FilterList::grammar);
}

/// Link `clip-path`'s grammar (and the path grammar its `path()` reads).
pub fn link_clip_path() {
    #[cfg(not(test))]
    let _ = CLIP_PATH.set(ClipPath::grammar);
}

/// `css` by `filter`'s grammar, once linked.
pub(crate) fn filter(css: &str) -> Option<FilterList> {
    #[cfg(not(test))]
    return FILTER.get().and_then(|grammar| grammar(css));
    #[cfg(test)]
    FilterList::grammar(css)
}

/// `css` by `clip-path`'s grammar, once linked.
pub(crate) fn clip_path(css: &str) -> Option<ClipPath> {
    #[cfg(not(test))]
    return CLIP_PATH.get().and_then(|grammar| grammar(css));
    #[cfg(test)]
    ClipPath::grammar(css)
}
