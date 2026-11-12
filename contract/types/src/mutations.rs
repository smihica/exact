//! A mutation's declaration: what it refreshes, and whether its mode takes
//! its `then`.

use super::{Sink, TypeError};
use contract_syntax::{Component, MutationDecl, MutationMode};

/// Check mutation `m` of `c`.
pub(crate) fn check(c: &Component, m: &MutationDecl, sink: &mut Sink) {
    // @ref LLP 1161.000 D2 — a parallel mutation's replies land in any
    // order, so a `then` some of them armed would lose the others.
    if let (MutationMode::Parallel, Some((_, span))) = (m.mode, &m.then) {
        sink.push(TypeError {
            id: "type-parallel-then",
            message: format!(
                "`{}` is `parallel`, which takes no `then`: its replies land in any order. Show a write through `refreshes`, or declare it `queue` to handle each reply in order",
                m.name
            ),
            span: *span,
        });
    }
    // @ref LLP 1054.000.000 D1 — what a send to this mutation refreshes.
    for (i, (name, span)) in m.refreshes.iter().enumerate() {
        if !c.resources.iter().any(|r| &r.name == name) {
            sink.push(TypeError {
                id: "type-refreshes-not-resource",
                message: format!(
                    "`{name}` is not a resource: `refreshes` names this component's resources"
                ),
                span: *span,
            });
        } else if m.refreshes[..i].iter().any(|(n, _)| n == name) {
            sink.push(TypeError {
                id: "type-refreshes-duplicate",
                message: format!("`{name}` is already named in `refreshes`"),
                span: *span,
            });
        }
    }
}
