//! @ref LLP 1092 D9 — a gated task's gate and key are read at commits, not
//! as the clock moves: one that reads `performanceNow()`, directly, through a derive
//! or through a `fn` body, is refused (`analyze-task-gate-clock`).

use super::AnalyzeError;
use contract_syntax::{Component, Expr, FnDecl, TemplatePart};
use std::collections::BTreeSet;

/// Every gate or key of `c`'s tasks that reads the clock.
pub(super) fn check(c: &Component, fns: &[FnDecl]) -> Vec<AnalyzeError> {
    let mut errors = Vec::new();
    for t in &c.tasks {
        for (what, e) in [("gate", &t.gate), ("key", &t.key)] {
            let Some(e) = e else { continue };
            let mut seen = BTreeSet::new();
            if let Some(at) = clock(e, c, fns, &mut seen, &BTreeSet::new()) {
                errors.push(AnalyzeError {
                    id: "analyze-task-gate-clock",
                    message: format!(
                        "`task {}`'s {what} reads `performanceNow()`{}. A gate is read at commits, not as the clock moves, so an `after` gated on `performanceNow()` cannot be dropped before it fires, and an `every` stops up to an interval late. Gate on state (`toast != \"\"`) and let `after(5000, …)` measure the time",
                        t.name,
                        match at {
                            Through::Direct => String::new(),
                            Through::Derive(d) => format!(" through `{d}`"),
                            Through::Fn(f) => format!(" through `fn {f}`"),
                        }
                    ),
                    span: e.span(),
                    related: Vec::new(),
                });
            }
        }
    }
    // @ref LLP 1125 D2 — a sequence's gate and key as a task's; its `play`
    // arguments are re-read after every commit, not as the clock moves, so
    // one that reads the clock would change at commits only.
    for q in &c.sequences {
        let options = q
            .options
            .iter()
            .map(|(name, e, _)| (name.as_str(), Some(e)));
        let gates = [("gate", q.gate.as_ref()), ("key", q.key.as_ref())];
        let play = [("tracks", Some(&q.tracks))].into_iter().chain(options);
        for (what, e) in gates.into_iter().chain(play) {
            let Some(e) = e else { continue };
            let mut seen = BTreeSet::new();
            let Some(at) = clock(e, c, fns, &mut seen, &BTreeSet::new()) else {
                continue;
            };
            let gate = matches!(what, "gate" | "key");
            let through = match at {
                Through::Direct => String::new(),
                Through::Derive(d) => format!(" through `{d}`"),
                Through::Fn(f) => format!(" through `fn {f}`"),
            };
            errors.push(AnalyzeError {
                id: if gate { "analyze-task-gate-clock" } else { "analyze-sequence-clock" },
                message: if gate {
                    format!("`sequence {}`'s {what} reads `performanceNow()`{through}. A gate is read at commits, not as the clock moves; gate on state (`playing`)", q.name)
                } else {
                    format!("`sequence {}`'s `{what}` reads `performanceNow()`{through}. `play`'s arguments are re-read at commits, not as the clock moves; the runner times each step from the beat, so say where in the pattern a step falls (`beat=`), not when", q.name)
                },
                span: e.span(),
                related: Vec::new(),
            });
        }
    }
    errors
}

enum Through {
    Direct,
    Derive(String),
    Fn(String),
}

/// Where `e` reads the clock, if it does: `performanceNow()` itself, or the first
/// derive or `fn` whose value does.
fn clock<'a>(
    e: &'a Expr,
    c: &'a Component,
    fns: &'a [FnDecl],
    seen: &mut BTreeSet<(&'static str, &'a str)>,
    bound: &BTreeSet<&'a str>,
) -> Option<Through> {
    match e {
        Expr::Call(name, args, _)
            if name == "performanceNow"
                && args.is_empty()
                && !fns.iter().any(|f| f.name == "performanceNow") =>
        {
            Some(Through::Direct)
        }
        Expr::Call(name, args, _) => {
            if let Some(f) = fns.iter().find(|f| &f.name == name) {
                if seen.insert(("fn", &f.name)) {
                    let locals = f.params.iter().map(|p| p.name.as_str()).collect();
                    if clock(&f.body, c, fns, seen, &locals).is_some() {
                        return Some(Through::Fn(shown(name)));
                    }
                }
            }
            args.iter().find_map(|a| clock(a, c, fns, seen, bound))
        }
        Expr::Ident(name, _) if !bound.contains(name.as_str()) => {
            if seen.insert(("derive", name)) {
                if let Some(d) = c.derives.iter().find(|d| d.name == *name) {
                    // A derive's inputs belong to its declaration, not the caller's locals.
                    return clock(&d.expr, c, fns, seen, &BTreeSet::new())
                        .map(|_| Through::Derive(shown(name)));
                }
            }
            None
        }
        Expr::Template(parts, _) => parts.iter().find_map(|p| match p {
            TemplatePart::Expr(e) => clock(e, c, fns, seen, bound),
            TemplatePart::Text(_) => None,
        }),
        Expr::Some(a, _)
        | Expr::Member(a, _, _)
        | Expr::NamedArg(_, a, _)
        | Expr::Unary(_, a, _)
        | Expr::Typed(a, _, _) => clock(a, c, fns, seen, bound),
        Expr::Arrow { params, body, .. } => {
            let mut locals = bound.clone();
            locals.extend(params.iter().map(String::as_str));
            clock(body, c, fns, seen, &locals)
        }
        Expr::Binary(_, a, b, _) => {
            clock(a, c, fns, seen, bound).or_else(|| clock(b, c, fns, seen, bound))
        }
        Expr::Let {
            name, value, body, ..
        } => clock(value, c, fns, seen, bound).or_else(|| {
            let mut locals = bound.clone();
            locals.insert(name);
            clock(body, c, fns, seen, &locals)
        }),
        Expr::Ternary(a, b, d, _) => clock(a, c, fns, seen, bound)
            .or_else(|| clock(b, c, fns, seen, bound))
            .or_else(|| clock(d, c, fns, seen, bound)),
        Expr::Match {
            subject,
            var,
            some,
            none,
            ..
        } => clock(subject, c, fns, seen, bound)
            .or_else(|| {
                let mut locals = bound.clone();
                locals.insert(var);
                clock(some, c, fns, seen, &locals)
            })
            .or_else(|| clock(none, c, fns, seen, bound)),
        Expr::List(items, _) => items.iter().find_map(|e| clock(e, c, fns, seen, bound)),
        Expr::Ident(..) | Expr::Number(..) | Expr::Str(..) | Expr::Bool(..) | Expr::None(_) => None,
    }
}

/// A name as written: a lifted child's `name#N` without its suffix.
fn shown(name: &str) -> String {
    name.split('#').next().unwrap_or(name).to_string()
}
