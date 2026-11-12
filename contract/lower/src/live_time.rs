//! HTML instants and runner-owned presentations. @ref LLP 1126 D6
use crate::{err, LowerError};
use contract_syntax::{Attr, Expr, Span};

pub(crate) const INSTANT_REFUSAL: &str = "`datetime` is an instant in epoch milliseconds; \
    bind a source's numeric field, e.g. `datetime=event.at` (parse date strings in the data module). \
    For the current instant, use `datetime=(wall.epochAtZero + performanceNow())`, \
    where `wall` is an `exactTime()` resource (see `contract vocab date`)";

pub(crate) fn check(tag: &str, attrs: &[Attr], span: Span) -> Result<(), LowerError> {
    let has = |name: &str| attrs.iter().any(|a| a.name == name);
    for a in attrs {
        if (matches!(a.name.as_str(), "datetime" | "format") && tag != "time")
            || (matches!(a.name.as_str(), "start" | "end") && tag != "progress")
        {
            return err(
                "lower-attr-tag",
                format!(
                    "`{}` on `{tag}` belongs to {}",
                    a.name,
                    if matches!(a.name.as_str(), "datetime" | "format") {
                        "`time`"
                    } else {
                        "an interval `progress`"
                    }
                ),
                a.span,
            );
        }
        if a.name == "datetime" && matches!(&a.value, Expr::Str(_, _)) {
            return err("lower-time-instant", INSTANT_REFUSAL, a.span);
        }
        if a.name == "format"
            && !matches!(&a.value, Expr::Str(v, _) if exact_kernel::time::FORMATS.contains(&v.as_str()))
        {
            return err(
                "lower-time-format",
                format!(
                    "`format` is a named presentation: {}",
                    exact_kernel::time::FORMATS.join(", ")
                ),
                a.span,
            );
        }
    }
    if tag == "time" && !has("datetime") {
        return err(
            "lower-time-instant",
            "`time` needs `datetime`, an instant in epoch milliseconds",
            span,
        );
    }
    if tag == "progress"
        && (has("start") || has("end"))
        && (!has("start") || !has("end") || has("value") || has("max"))
    {
        return err(
            "lower-progress-interval",
            "write `progress start=… end=…` or `progress value=… max=…`",
            span,
        );
    }
    if tag == "progress" {
        let literal = |name| {
            attrs
                .iter()
                .find(|a| a.name == name)
                .and_then(|a| crate::values::numeric_literal(&a.value))
        };
        if literal("start")
            .zip(literal("end"))
            .is_some_and(|(a, b)| b <= a)
        {
            return err(
                "lower-progress-interval",
                "write `progress start=… end=…` with `end` after `start`",
                span,
            );
        }
    }
    Ok(())
}
