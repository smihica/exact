use crate::LowerError;
use contract_syntax::{Expr, Stmt};
use contract_types::Checked;

/// `lower-select-after-push`: `select` of a router a `go` or `push` of the
/// same action made, directly or through a `let`. By LLP 1038 §3's verb
/// table, `push` adds the visit to the selected tab's stack, whatever tab
/// declares its route, and so does `go` unless the location is on the
/// selected stack (it pops to it) or another tab's top (it selects that tab).
/// A `select` after either is wrong both ways: on a push the visit waits on
/// the tab left, a deep link into another tab that opens nothing (the euxy
/// port); where `go` selected the named tab, selecting it again pops it to
/// its root.
/// It warns only where the router is surely pushed: a choice only when
/// every arm is, a name only while no nearer binder hides the `let`.
pub(super) fn select_after_push(checked: &Checked<'_>) -> Vec<LowerError> {
    let Some(decl) = &checked.file.routes else {
        return Vec::new();
    };
    // A `fn` of the app's by one of these names is called in the router's place.
    let fns = &checked.types.shapes.fns;
    if ["go", "push", "select"]
        .iter()
        .any(|f| fns.contains_key(*f))
    {
        return Vec::new();
    }
    let mut out = Vec::new();
    for action in &checked.expanded.root.actions {
        let mut scope: Bound = action
            .params
            .iter()
            .map(|p| (p.name.as_str(), None))
            .collect();
        // A same-top no-op needs the builtin, not an app function named `top`.
        if fns.contains_key("top") {
            scope.push(("top", None));
        }
        pushed_block(&action.body, &decl.slot, &mut scope, &mut out);
    }
    out
}

/// The names in scope, nearest last, each with the verb, `go` or `push`,
/// that made the router it holds, if one surely did.
type Bound<'a> = Vec<(&'a str, Option<&'a str>)>;

/// A block's statements, its `let`s and `case some(…)` binders scoped to it.
fn pushed_block<'a>(body: &'a [Stmt], nav: &str, scope: &mut Bound<'a>, out: &mut Vec<LowerError>) {
    let depth = scope.len();
    for s in body {
        match s {
            Stmt::Let { name, expr, .. } => {
                selects(expr, nav, scope, out);
                let verb = pushes(expr, scope);
                scope.push((name, verb));
            }
            Stmt::Assign { expr, .. } => selects(expr, nav, scope, out),
            Stmt::Command { args, .. } | Stmt::Send { args, .. } => {
                for a in args {
                    selects(a, nav, scope, out);
                }
            }
            Stmt::Refresh { .. } => {}
            Stmt::Call { body, .. } => pushed_block(body, nav, scope, out),
            Stmt::If {
                cond,
                then,
                otherwise,
                ..
            } => {
                selects(cond, nav, scope, out);
                pushed_block(then, nav, scope, out);
                pushed_block(otherwise, nav, scope, out);
            }
            Stmt::Match {
                subject,
                some,
                none,
                ..
            } => {
                selects(subject, nav, scope, out);
                scope.push((&some.0, None));
                pushed_block(&some.1, nav, scope, out);
                scope.pop();
                pushed_block(none, nav, scope, out);
            }
        }
    }
    scope.truncate(depth);
}

/// The verb, `go` or `push`, that surely made a router value: a choice's
/// only when every arm's is.
fn pushes<'a>(e: &'a Expr, scope: &Bound<'a>) -> Option<&'a str> {
    match e {
        Expr::Call(f, args, _) if f == "go" || f == "push" => match args.as_slice() {
            [router, location] if current_top(router, location, scope) => pushes(router, scope),
            _ => Some(f),
        },
        Expr::Ident(n, _) => scope.iter().rev().find(|(p, _)| p == n)?.1,
        Expr::Ternary(_, a, b, _) => pushes(b, scope).and(pushes(a, scope)),
        _ => None,
    }
}

fn current_top(router: &Expr, location: &Expr, scope: &Bound<'_>) -> bool {
    let (Expr::Ident(router, _), Expr::Member(base, field, _)) = (router, location) else {
        return false;
    };
    let Expr::Call(f, args, _) = &**base else {
        return false;
    };
    field == "url"
        && f == "top"
        && !scope.iter().any(|(name, _)| *name == "top")
        && matches!(args.as_slice(), [Expr::Ident(top_router, _)] if top_router == router)
}

/// Each `select` in `e` of a router a `go` or `push` made.
fn selects<'a>(e: &'a Expr, nav: &str, scope: &mut Bound<'a>, out: &mut Vec<LowerError>) {
    match e {
        Expr::Call(f, args, span) if f == "select" => {
            if let [router, tab] = args.as_slice() {
                if let Some(verb) = pushes(router, scope) {
                    let tab = match tab {
                        Expr::Str(t, _) => format!("\"{t}\""),
                        _ => "tab".into(),
                    };
                    let why = if verb == "go" {
                        format!("where `go` pushes, it pushes onto the selected tab's stack, whatever tab declares the route, so the visit stays on the tab being left; where the location is already {tab}'s top, `go` selects {tab} itself and this `select` pops it to its root")
                    } else {
                        format!("`{verb}` adds its visit to the selected tab's stack, whatever tab declares the route, so the visit stays on the tab being left")
                    };
                    out.push(LowerError {
                        id: "lower-select-after-push",
                        message: format!("this `select` does not land the visit on {tab}: {why}. To open a location on its route's declared tab, write `open({nav}, url)`, which puts the route's declared chain in place of that tab's stack (docs/recipes/tabs.md)"),
                        span: *span,
                    });
                }
            }
        }
        // A `let` of a push names the push.
        Expr::Let {
            name, value, body, ..
        } => {
            selects(value, nav, scope, out);
            let verb = pushes(value, scope);
            scope.push((name, verb));
            selects(body, nav, scope, out);
            scope.pop();
            return;
        }
        _ => {}
    }
    // Another binder hides an outer name of its own for its body.
    for (x, binds) in e.scoped_operands() {
        let depth = scope.len();
        scope.extend(binds.iter().map(|name| (name, None)));
        selects(x, nav, scope, out);
        scope.truncate(depth);
    }
}
