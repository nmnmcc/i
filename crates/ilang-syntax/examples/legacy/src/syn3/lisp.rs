use crate::lisp::{Datum, DatumKind as K};
use crate::span::Span;
pub use syntax3::syn3::lisp::*;
fn round<'a>(d: &'a Datum, head: &str) -> Option<&'a [Datum]> {
    match &d.kind {
        K::Round(parts) => match parts.first().map(|p| &p.kind) {
            Some(K::Symbol(s)) if s == head => Some(&parts[1..]),
            _ => None,
        },
        _ => None,
    }
}

fn symbol(d: &Datum) -> Option<&str> {
    match &d.kind {
        K::Symbol(s) => Some(s),
        _ => None,
    }
}

fn sym(s: &str, span: Span) -> Datum {
    Datum {
        kind: K::Symbol(s.into()),
        span,
        docs: Vec::new(),
    }
}

fn node(kind: K, span: Span) -> Datum {
    Datum {
        kind,
        span,
        docs: Vec::new(),
    }
}

/// Rewrites a quoted datum written with the older Lisp rows to the rows of
/// §6.8, as far as the datum shows its position: functions get data-form
/// parameters and a service pattern, function types a data type and needs,
/// clause blocks become chains, the pipe and the match get the head `|>`,
/// and records become data. Labels `(: x e)` in calls and environment
/// statements `(! N e)` are left, because a datum does not show whether
/// `(: x T)` is a label or a typed pattern, nor where a block ends.
pub fn convert_quote(d: &Datum) -> Datum {
    let span = d.span;
    let all = |xs: &[Datum]| xs.iter().map(convert_quote).collect::<Vec<_>>();
    let is_clause = |c: &Datum| round(c, "->").is_some_and(|r| r.len() == 2);
    let clause = |c: &Datum| -> Datum {
        let [head, body] = round(c, "->").unwrap() else {
            unreachable!()
        };
        let (pat, guard) = match round(head, "?") {
            Some([p, g]) => (p, Some(g)),
            _ => (head, None),
        };
        let data = node(
            K::Round(vec![sym(",", pat.span), convert_quote(pat)]),
            pat.span,
        );
        let params = match guard {
            Some(g) => node(
                K::Round(vec![sym("?", head.span), data, convert_quote(g)]),
                head.span,
            ),
            None => data,
        };
        node(
            K::Round(vec![
                sym("->", c.span),
                params,
                node(K::Discard("_".into()), c.span),
                convert_quote(body),
            ]),
            c.span,
        )
    };
    let chain = |cs: &[Datum], span: Span| -> Datum {
        if let [one] = cs {
            return clause(one);
        }
        let mut parts = vec![sym("|", span)];
        parts.extend(cs.iter().map(clause));
        node(K::Round(parts), span)
    };
    let kind = match &d.kind {
        K::Curly(parts) if !parts.is_empty() && parts.iter().all(is_clause) => {
            return chain(parts, span);
        }
        K::Curly(parts)
            if parts
                .first()
                .is_some_and(|p| round(p, ":").is_some() || symbol(p) == Some("..")) =>
        {
            // A structural record.
            let mut out = vec![sym(",", span)];
            for p in parts {
                match round(p, ":") {
                    Some([l, v]) => out.push(node(
                        K::Round(vec![sym("=", p.span), l.clone(), convert_quote(v)]),
                        p.span,
                    )),
                    _ => out.push(convert_quote(p)),
                }
            }
            K::Round(out)
        }
        K::Round(parts) => match (parts.first().and_then(symbol), &parts[..]) {
            (Some("->"), [arrow, params, ret, body]) if matches!(params.kind, K::Round(_)) => {
                // A function: data-form parameters; the environment
                // parameters become the service pattern; the `^` and `!`
                // parts go (the needs are inferred).
                let K::Round(ps) = &params.kind else {
                    unreachable!()
                };
                let mut items = vec![sym(",", params.span)];
                let mut services = Vec::new();
                for p in ps {
                    match round(p, "!") {
                        Some([x, n]) => services.push(node(
                            K::Round(vec![sym(":", p.span), x.clone(), convert_quote(n)]),
                            p.span,
                        )),
                        _ => items.push(convert_quote(p)),
                    }
                }
                let (value, _, _) = split_result(ret);
                let mut result = convert_quote(value);
                if !services.is_empty() {
                    let mut group = vec![sym(",", ret.span)];
                    group.extend(services);
                    result = node(
                        K::Round(vec![
                            sym("<-", ret.span),
                            result,
                            node(K::Round(group), ret.span),
                        ]),
                        ret.span,
                    );
                }
                K::Round(vec![
                    arrow.clone(),
                    node(K::Round(items), params.span),
                    result,
                    convert_quote(body),
                ])
            }
            (Some("->"), [arrow, params, ret]) if matches!(&params.kind, K::Round(ps) if ps.first().and_then(symbol).is_none()) =>
            {
                // A function type: a data type with no labels, and the needs.
                let K::Round(ps) = &params.kind else {
                    unreachable!()
                };
                let mut items = vec![sym(",", params.span)];
                for p in ps {
                    match round(p, ":") {
                        Some([_, t]) => items.push(convert_quote(t)),
                        _ => items.push(convert_quote(p)),
                    }
                }
                let (value, error, env) = split_result(ret);
                let mut needs = Vec::new();
                if let Some(e) = error
                    && !matches!(&e.kind, K::UpName(s) if s == "Never")
                {
                    needs.push(node(
                        K::Round(vec![
                            node(K::UpName("Fail".into()), e.span),
                            convert_quote(e),
                        ]),
                        e.span,
                    ));
                }
                if let Some(v) = env {
                    match round(v, "|") {
                        Some(ms) => needs.extend(ms.iter().map(convert_quote)),
                        None => needs.push(convert_quote(v)),
                    }
                }
                let mut result = convert_quote(value);
                if !needs.is_empty() {
                    let mut group = vec![sym(",", ret.span)];
                    group.extend(needs);
                    result = node(
                        K::Round(vec![
                            sym("<-", ret.span),
                            result,
                            node(K::Round(group), ret.span),
                        ]),
                        ret.span,
                    );
                }
                K::Round(vec![
                    arrow.clone(),
                    node(K::Round(items), params.span),
                    result,
                ])
            }
            (Some("|" | "."), [_, subject, block]) if matches!(&block.kind, K::Curly(cs) if !cs.is_empty() && cs.iter().all(is_clause)) =>
            {
                let K::Curly(cs) = &block.kind else {
                    unreachable!()
                };
                K::Round(vec![
                    sym("|>", span),
                    convert_quote(subject),
                    chain(cs, block.span),
                ])
            }
            (Some("|"), [_, lhs, rhs]) if !matches!(lhs.kind, K::UpName(_)) => K::Round(vec![
                sym("|>", span),
                convert_quote(lhs),
                convert_quote(rhs),
            ]),
            _ => K::Round(all(parts)),
        },
        K::Square(parts) => K::Square(all(parts)),
        K::Curly(parts) => K::Curly(all(parts)),
        K::Bits(parts) => K::Bits(all(parts)),
        K::Quote(x) => K::Quote(Box::new(convert_quote(x))),
        K::Unquote(x) => K::Unquote(x.clone()),
        K::Splice(x) => K::Splice(x.clone()),
        other => other.clone(),
    };
    node(kind, span)
}

/// The parts of the result of an older function or function type: `R`,
/// `(^ R E)`, `(! R V)`, `(! (^ R E) V)`.
fn split_result(ret: &Datum) -> (&Datum, Option<&Datum>, Option<&Datum>) {
    let (ret, env) = match round(ret, "!") {
        Some([r, v]) => (r, Some(v)),
        _ => (ret, None),
    };
    match round(ret, "^") {
        Some([r, e]) => (r, Some(e), env),
        _ => (ret, None, env),
    }
}

/// The reverse of [`convert_quote`]: a quoted datum in the rows of §6.8 is
/// rewritten to the older rows, which the macro engine of this stage reads
/// (the transition front keeps the current tree, so the syntax that a macro
/// sees and gives is in the older rows).
pub fn unconvert_quote(d: &Datum) -> Datum {
    let span = d.span;
    let all = |xs: &[Datum]| xs.iter().map(unconvert_quote).collect::<Vec<_>>();
    // `(-> (, p) _ e)` or `(-> (? (, p) g) _ e)`: a branch of one position.
    fn branch(c: &Datum) -> Option<(&Datum, Option<&Datum>, &Datum)> {
        let [params, ret, body] = round(c, "->")? else {
            return None;
        };
        if !matches!(ret.kind, K::Discard(_)) {
            return None;
        }
        let (data, guard) = match round(params, "?") {
            Some([data, g]) => (data, Some(g)),
            _ => (params, None),
        };
        match round(data, ",") {
            Some([p]) => Some((p, guard, body)),
            _ => None,
        }
    }
    let clause = |c: &Datum| -> Datum {
        let (p, guard, body) = branch(c).unwrap();
        let head = match guard {
            Some(g) => node(
                K::Round(vec![
                    sym("?", p.span),
                    unconvert_quote(p),
                    unconvert_quote(g),
                ]),
                p.span,
            ),
            None => unconvert_quote(p),
        };
        node(
            K::Round(vec![sym("->", c.span), head, unconvert_quote(body)]),
            c.span,
        )
    };
    // The result of a function or a function type: `(<- R (, needs…))`.
    let needs = |ret: &Datum| -> (Datum, Vec<Datum>) {
        match round(ret, "<-") {
            Some([r, group]) => match round(group, ",") {
                Some(items) => (unconvert_quote(r), items.to_vec()),
                None => (unconvert_quote(r), vec![group.clone()]),
            },
            _ => (unconvert_quote(ret), Vec::new()),
        }
    };
    let kind = match &d.kind {
        K::Round(parts) => match (parts.first().and_then(symbol), &parts[..]) {
            (Some("->"), [arrow, params, ret, body]) if round(params, ",").is_some() => {
                let mut ps = all(round(params, ",").unwrap());
                let (result, services) = needs(ret);
                for s in services {
                    match round(&s, ":") {
                        Some([x, n]) => ps.push(node(
                            K::Round(vec![sym("!", s.span), x.clone(), unconvert_quote(n)]),
                            s.span,
                        )),
                        _ => ps.push(unconvert_quote(&s)),
                    }
                }
                K::Round(vec![
                    arrow.clone(),
                    node(K::Round(ps), params.span),
                    result,
                    unconvert_quote(body),
                ])
            }
            (Some("->"), [arrow, params, ret]) if round(params, ",").is_some() => {
                let ps = all(round(params, ",").unwrap());
                let (mut result, items) = needs(ret);
                let mut env = Vec::new();
                let mut error = None;
                for item in items {
                    match &item.kind {
                        K::Round(xs)
                            if xs.len() == 2
                                && matches!(&xs[0].kind, K::UpName(s) if s == "Fail") =>
                        {
                            error = Some(unconvert_quote(&xs[1]));
                        }
                        _ => env.push(unconvert_quote(&item)),
                    }
                }
                if let Some(e) = error {
                    result = node(K::Round(vec![sym("^", ret.span), result, e]), ret.span);
                }
                if !env.is_empty() {
                    let v = if env.len() == 1 {
                        env.pop().unwrap()
                    } else {
                        let mut parts = vec![sym("|", ret.span)];
                        parts.extend(env);
                        node(K::Round(parts), ret.span)
                    };
                    result = node(K::Round(vec![sym("!", ret.span), result, v]), ret.span);
                }
                K::Round(vec![arrow.clone(), node(K::Round(ps), params.span), result])
            }
            (Some("|>"), [_, subject, rhs]) => {
                let clauses: Option<Vec<&Datum>> = match round(rhs, "|") {
                    Some(cs) if !cs.is_empty() && cs.iter().all(|c| branch(c).is_some()) => {
                        Some(cs.iter().collect())
                    }
                    _ if branch(rhs).is_some() => Some(vec![rhs]),
                    _ => None,
                };
                match clauses {
                    Some(cs) => K::Round(vec![
                        sym("|", span),
                        unconvert_quote(subject),
                        node(K::Curly(cs.into_iter().map(clause).collect()), rhs.span),
                    ]),
                    None => K::Round(vec![
                        sym("|", span),
                        unconvert_quote(subject),
                        unconvert_quote(rhs),
                    ]),
                }
            }
            (Some("|"), [_, cs @ ..]) if cs.len() > 1 && cs.iter().all(|c| branch(c).is_some()) => {
                K::Curly(cs.iter().map(clause).collect())
            }
            (Some(","), [_, items @ ..])
                if items
                    .first()
                    .is_some_and(|p| round(p, "=").is_some() || symbol(p) == Some("..")) =>
            {
                let mut out = Vec::new();
                for p in items {
                    match round(p, "=") {
                        Some([l, v]) => out.push(node(
                            K::Round(vec![sym(":", p.span), l.clone(), unconvert_quote(v)]),
                            p.span,
                        )),
                        _ => out.push(unconvert_quote(p)),
                    }
                }
                K::Curly(out)
            }
            _ => K::Round(all(parts)),
        },
        K::Square(parts) => K::Square(all(parts)),
        K::Curly(parts) => K::Curly(all(parts)),
        K::Bits(parts) => K::Bits(all(parts)),
        K::Quote(x) => K::Quote(Box::new(unconvert_quote(x))),
        K::Unquote(x) => K::Unquote(x.clone()),
        K::Splice(x) => K::Splice(x.clone()),
        other => other.clone(),
    };
    node(kind, span)
}
