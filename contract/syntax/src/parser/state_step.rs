//! State assertions read literals or two paths from one agent reply.
use super::*;

impl Parser {
    /// A state's name, then fields and list indexes at any depth, joined by
    /// `.` (`gateway.state`, `rows.0`: feed F10, drums R7).
    pub(super) fn state_path(&mut self) -> R<String> {
        let (mut name, _) = self.ident()?;
        while self.eat_punct(".") {
            name.push('.');
            match self.peek_kind().clone() {
                TokenKind::Number(n)
                    if n.fract() == 0.0 && (0.0..=u32::MAX as f64).contains(&n) =>
                {
                    self.next();
                    name.push_str(&format!("{}", n as u64));
                }
                TokenKind::Ident(_) => name.push_str(&self.ident()?.0),
                other => {
                    return self.err(
                        "syntax-expected-name",
                        format!(
                            "expected a field or a list index after `.`, found {}",
                            describe(&other)
                        ),
                    );
                }
            }
        }
        Ok(name)
    }

    pub(super) fn expect_state(&mut self, span: Span) -> R<Step> {
        let name = self.state_path()?;
        let comparison = if self.eat_punct("==") {
            StateComparison::Equal
        } else if self.eat_punct("!=") {
            StateComparison::NotEqual
        } else if self.eat_punct(">=") {
            StateComparison::AtLeast
        } else if self.eat_punct("<=") {
            StateComparison::AtMost
        } else if self.at_ident("in") {
            self.next();
            StateComparison::In
        } else {
            return self.err(
                "syntax-expected-step",
                "`expect state` takes `==`, `!=`, `>=`, `<=`, or `in`",
            );
        };
        let value = if self.at_ident("state") && matches!(self.peek2(), TokenKind::Ident(_)) {
            self.next();
            StateOperand::State(self.state_path()?)
        } else {
            StateOperand::Literal(match comparison {
                StateComparison::AtLeast | StateComparison::AtMost => {
                    Expr::Number(self.step_number("a numeric bound")?, span)
                }
                StateComparison::In => {
                    let mut values = vec![self.state_literal()?];
                    while !matches!(
                        self.peek_kind(),
                        TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof
                    ) {
                        values.push(self.state_literal()?);
                    }
                    Expr::List(values, span)
                }
                StateComparison::Equal | StateComparison::NotEqual => self.state_literal()?,
            })
        };
        if !matches!(
            self.peek_kind(),
            TokenKind::Newline | TokenKind::Dedent | TokenKind::Eof
        ) {
            return self.err(
                "syntax-expected-step",
                "`expect state` compares literals or `state <path>`, not expressions",
            );
        }
        Ok(Step::ExpectState {
            name,
            comparison,
            value,
            span,
        })
    }
}
