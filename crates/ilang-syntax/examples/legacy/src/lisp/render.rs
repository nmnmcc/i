//! Stack-based normal-surface renderer for Lisp datums.
use super::to_source::{
    atom, bad, ends_in_type_test, import_name, is_binop, is_clause, is_match, is_upper_head, label,
    round, symbol,
};
use super::{Datum, DatumKind as K, show};
use crate::Diagnostic;

type Result<T> = std::result::Result<T, Diagnostic>;
#[derive(Clone, Copy)]
pub enum P {
    Import,
    ImportPart,
    Export,
    Item,
    Head,
    Constructor,
    Field,
    Attribute,
    Name,
    Type,
    Return,
    Arg,
    Expr,
    Target,
    Operand,
    Segment,
    Format,
    Parameter,
    Block,
    Statement,
    Clause,
    ClauseHead,
    Pattern,
    PatternSegment,
}
enum Step<'a> {
    Node(P, &'a Datum),
    Text(String),
    Boundary,
}

pub fn render(d: &Datum, pos: P) -> Result<String> {
    render_anchored(d, pos).map(|(text, _)| text)
}

pub fn render_anchored(
    d: &Datum,
    pos: P,
) -> Result<(String, Vec<(crate::span::Span, usize)>)> {
    let mut out = String::new();
    let mut anchors = Vec::new();
    let mut boundary = false;
    let mut stack = vec![Step::Node(pos, d)];
    while let Some(step) = stack.pop() {
        let (p, d) = match step {
            Step::Text(s) => {
                if boundary && !s.is_empty() {
                    out.push(if matches!(s.chars().next(), Some('(' | '{' | '-')) {
                        '\n'
                    } else {
                        ' '
                    });
                    boundary = false;
                }
                out.push_str(&s);
                continue;
            }
            Step::Boundary => {
                boundary = true;
                continue;
            }
            Step::Node(p, d) => (p, d),
        };
        if matches!(p, P::Statement) {
            anchors.push((d.span, out.len() + usize::from(boundary)));
        }
        let mut tasks = Vec::new();
        macro_rules! text {
            ($v:expr) => {
                tasks.push(Step::Text(($v).into()))
            };
        }
        macro_rules! node {
            ($p:ident, $v:expr) => {
                tasks.push(Step::Node(P::$p, $v))
            };
        }
        macro_rules! seq {
            ($xs:expr, $p:ident, $sep:expr) => {{
                for (i, v) in $xs.iter().enumerate() {
                    if i != 0 {
                        text!($sep);
                    }
                    node!($p, v);
                }
            }};
        }
        macro_rules! lines {
            ($xs:expr, $p:ident) => {{
                for (i, v) in $xs.iter().enumerate() {
                    if i != 0 {
                        tasks.push(Step::Boundary);
                    }
                    node!($p, v);
                }
            }};
        }
        macro_rules! sequence {
            ($xs:expr, $p:ident) => {{
                let mut i = 0;
                let xs = $xs;
                let mut first = true;
                while i < xs.len() {
                    if !first {
                        text!(", ");
                    }
                    first = false;
                    if symbol(&xs[i], "..") {
                        text!("..");
                        i += 1;
                        if i == xs.len() {
                            break;
                        }
                    }
                    node!($p, &xs[i]);
                    i += 1;
                }
            }};
        }
        macro_rules! record {
            ($xs:expr, $kind:ident, $type_record:expr) => {{
                let xs = $xs;
                text!("(");
                let mut i = 0;
                let mut first = true;
                let has_base = xs.first().is_some_and(|x| symbol(x, ".."));
                if has_base {
                    if xs.len() < 2 {
                        return Err(bad(d, "a record spread"));
                    }
                    text!("..");
                    node!($kind, &xs[1]);
                    i = 2;
                    first = false;
                }
                while i < xs.len() {
                    if !first {
                        text!(", ");
                    }
                    first = false;
                    match round(&xs[i]) {
                        Some([h, n, v]) if symbol(h, ":") => {
                            match &n.kind {
                                K::Name(_) | K::String(_) => text!(show(n)),
                                K::UpName(_) if $type_record => text!(show(n)),
                                _ => return Err(bad(n, "a record label")),
                            }
                            text!(": ");
                            node!($kind, v);
                        }
                        Some([h, n]) if $type_record && has_base && symbol(h, "-") => {
                            text!("-");
                            match &n.kind {
                                K::Name(_) | K::UpName(_) | K::String(_) => text!(show(n)),
                                _ => return Err(bad(n, "a record label")),
                            }
                        }
                        _ => return Err(bad(&xs[i], "a record field")),
                    }
                    i += 1;
                }
                text!(")");
            }};
        }
        match p {
            P::Import => {
                let xs = round(d).ok_or_else(|| bad(d, "an import"))?;
                match xs {
                    [h, rest @ ..] if symbol(h, "/") && rest.len() >= 2 => {
                        for (i, x) in rest.iter().enumerate() {
                            if i > 0 {
                                text!("/");
                            }
                            text!(label(x)?);
                        }
                    }
                    [h, a, b] if symbol(h, "=") => {
                        node!(ImportPart, a);
                        text!(" = ");
                        node!(Import, b);
                    }
                    [h, a, b, c] if symbol(h, "=") => {
                        text!(label(a)?);
                        text!(" = ");
                        node!(ImportPart, b);
                        text!(" = ");
                        node!(Import, c);
                    }
                    _ => return Err(bad(d, "an import")),
                }
            }
            P::ImportPart => {
                if let Some([h, rest @ ..]) = round(d)
                    && symbol(h, ",")
                    && !rest.is_empty()
                {
                    text!("(");
                    for (i, x) in rest.iter().enumerate() {
                        if i > 0 {
                            text!(", ");
                        }
                        text!(import_name(x)?);
                    }
                    text!(")");
                } else {
                    text!(label(d)?);
                }
            }
            P::Export => {
                if let Some([n, sp]) = round(d)
                    && symbol(sp, "..")
                {
                    text!(import_name(n)?);
                    text!("(..)");
                } else {
                    text!(import_name(d)?);
                }
            }
            P::Item => {
                let xs = round(d).ok_or_else(|| bad(d, "an item"))?;
                match xs {
                    [h, a, it] if symbol(h, "@") => {
                        text!("@");
                        node!(Attribute, a);
                        tasks.push(Step::Boundary);
                        node!(Item, it);
                    }
                    [h, l, r] if symbol(h, "=") => {
                        node!(Head, l);
                        text!(" = ");
                        if is_upper_head(l) {
                            node!(Type, r);
                        } else {
                            node!(Expr, r);
                        }
                    }
                    [h, l, t] if symbol(h, ":") => {
                        node!(Head, l);
                        text!(": ");
                        node!(Type, t);
                    }
                    [h, b] if matches!(&b.kind, K::Curly(_)) && is_upper_head(h) => {
                        let K::Curly(cs) = &b.kind else {
                            unreachable!()
                        };
                        node!(Head, h);
                        text!(" { ");
                        seq!(cs, Constructor, " ");
                        text!(" }");
                    }
                    [h, args @ ..] => {
                        node!(Name, h);
                        text!("(");
                        seq!(args, Arg, ", ");
                        text!(")");
                    }
                    _ => return Err(bad(d, "an item")),
                }
            }
            P::Head => {
                if let Some([h, l, t]) = round(d)
                    && symbol(h, ":")
                {
                    node!(Head, l);
                    text!(": ");
                    node!(Type, t);
                } else if let Some([h, args @ ..]) = round(d)
                    && matches!(h.kind, K::Name(_) | K::UpName(_))
                    && !args.is_empty()
                {
                    text!(show(h));
                    text!("(");
                    seq!(args, Type, ", ");
                    text!(")");
                } else {
                    node!(Pattern, d);
                }
            }
            P::Constructor => match &d.kind {
                K::UpName(s) => text!(s.clone()),
                K::Round(xs) if xs.len() > 1 && matches!(xs[0].kind, K::UpName(_)) => {
                    text!(show(&xs[0]));
                    text!("(");
                    seq!(&xs[1..], Field, ", ");
                    text!(")");
                }
                _ => return Err(bad(d, "a constructor")),
            },
            P::Field => {
                if let Some([h, n, t]) = round(d)
                    && symbol(h, ":")
                {
                    text!(label(n)?);
                    text!(": ");
                    node!(Type, t);
                } else {
                    node!(Type, d);
                }
            }
            P::Attribute => {
                if let Some([h, args @ ..]) = round(d)
                    && !symbol(h, ".")
                {
                    node!(Name, h);
                    text!("(");
                    seq!(args, Arg, ", ");
                    text!(")");
                } else {
                    node!(Name, d);
                }
            }
            P::Name => match &d.kind {
                K::Name(s) | K::UpName(s) => text!(s.clone()),
                K::Round(xs) if matches!(xs.as_slice(),[h,_,_] if symbol(h,".")) => {
                    text!(label(&xs[1])?);
                    text!(".");
                    text!(import_name(&xs[2])?);
                }
                _ => return Err(bad(d, "a name")),
            },
            P::Type => {
                if let Some(s) = atom(d) {
                    text!(s);
                } else if let Some(xs) = round(d) {
                    if is_record(xs) {
                        record!(xs, Type, true);
                    } else {
                        match xs {
                            // syntax-3 stage 6: `()` is the empty data type.
                            [] => text!("()"),
                            [h, a, b] if symbol(h, ".") => {
                                text!(label(a)?);
                                text!(".");
                                text!(import_name(b)?);
                            }
                            [h, a] if symbol(h, "-") && matches!(a.kind, K::Int(_)) => {
                                text!("-");
                                text!(show(a));
                            }
                            [h, rest @ ..] if symbol(h, ",") && !rest.is_empty() => {
                                text!("(");
                                seq!(rest, Type, ", ");
                                text!(")");
                            }
                            [h, rest @ ..]
                                if (symbol(h, "|") || symbol(h, "&") || symbol(h, "-"))
                                    && rest.len() >= 2 =>
                            {
                                text!("{ ");
                                seq!(rest, Type, &format!(" {} ", show(h)));
                                text!(" }");
                            }
                            [h, ps, r] if symbol(h, "->") => {
                                let ps =
                                    round(ps).ok_or_else(|| bad(ps, "function type parameters"))?;
                                text!("{ (");
                                seq!(ps, Field, ", ");
                                text!(") -> ");
                                node!(Return, r);
                                text!(" }");
                            }
                            [head]
                                if matches!(head.kind, K::Name(_))
                                    || matches!(round(head),Some([h,_,n]) if symbol(h,".")&&matches!(n.kind,K::Name(_))) =>
                            {
                                node!(Name, head);
                                text!("()");
                            }
                            [head, args @ ..] if !args.is_empty() => {
                                node!(Name, head);
                                text!("(");
                                seq!(args, Type, ", ");
                                text!(")");
                            }
                            _ => return Err(bad(d, "a type")),
                        }
                    }
                } else if let K::Curly(xs) = &d.kind
                    && xs.len() == 1
                {
                    text!("{ ");
                    node!(Type, &xs[0]);
                    text!(" }");
                } else {
                    return Err(bad(d, "a type"));
                }
            }
            P::Return => {
                if let Some([h, r, e]) = round(d)
                    && symbol(h, "^")
                {
                    if matches!(&r.kind,K::Discard(s) if s=="_")
                        && matches!(&e.kind,K::UpName(s) if s=="Never")
                    {
                        text!("_");
                    } else {
                        node!(Type, r);
                        text!(" ^ ");
                        node!(Type, e);
                    }
                } else if let Some([h, r, e]) = round(d)
                    && symbol(h, "!")
                {
                    node!(Return, r);
                    text!(" ! ");
                    node!(Type, e);
                } else {
                    node!(Type, d);
                }
            }
            P::Arg => {
                if let Some([h, n, v]) = round(d)
                    && symbol(h, ":")
                {
                    text!(label(n)?);
                    text!(": ");
                    node!(Expr, v);
                } else {
                    node!(Expr, d);
                }
            }
            P::Expr => {
                if let Some(s) = atom(d) {
                    text!(s);
                } else {
                    match &d.kind {
                        K::Round(xs) if is_record(xs) => record!(xs, Expr, false),
                        K::Round(xs) => match xs.as_slice() {
                            [] => text!("()"),
                            [h, rest @ ..] if symbol(h, ",") && !rest.is_empty() => {
                                text!("(");
                                seq!(rest, Expr, ", ");
                                text!(")");
                            }
                            [h, a, b] if symbol(h, ".") => {
                                if let K::Curly(cs) = &b.kind {
                                    node!(Operand, a);
                                    text!(" |> { ");
                                    lines!(cs, Clause);
                                    text!(" }");
                                } else {
                                    node!(Target, a);
                                    text!(".");
                                    text!(atom(b).ok_or_else(|| bad(b, "a field"))?);
                                }
                            }
                            [h, params, result, body] if symbol(h, "->") => {
                                let ps = round(params)
                                    .ok_or_else(|| bad(params, "function parameters"))?;
                                text!("(");
                                seq!(ps, Parameter, ", ");
                                text!(")");
                                if !matches!(&result.kind,K::Discard(s) if s=="_") {
                                    text!(" -> ");
                                    node!(Return, result);
                                }
                                text!(" ");
                                if matches!(body.kind, K::Curly(_)) {
                                    node!(Block, body);
                                } else {
                                    text!("{ ");
                                    node!(Expr, body);
                                    text!(" }");
                                }
                            }
                            [h, a] if symbol(h, "-") || symbol(h, "-.") => {
                                text!(show(h));
                                let negative = atom(a).is_some_and(|s| s.starts_with('-'))
                                    || matches!(round(a),Some([op,_]) if symbol(op,"-")||symbol(op,"-."));
                                if negative
                                    || is_match(a)
                                    || matches!(round(a), Some([op, _, _]) if is_binop(op))
                                {
                                    text!("{ ");
                                    node!(Expr, a);
                                    text!(" }");
                                } else {
                                    node!(Expr, a);
                                }
                            }
                            [h, a, b] if is_binop(h) => {
                                node!(Operand, a);
                                text!(" ");
                                text!(if symbol(h, "|") {
                                    "|>".to_owned()
                                } else {
                                    show(h)
                                });
                                text!(" ");
                                node!(Operand, b);
                            }
                            [head, spread, base, fields @ ..] if symbol(spread, "..") => {
                                node!(Target, head);
                                text!("(..");
                                node!(Expr, base);
                                for f in fields {
                                    text!(", ");
                                    node!(Arg, f);
                                }
                                text!(")");
                            }
                            [head, args @ ..] => {
                                node!(Target, head);
                                text!("(");
                                seq!(args, Arg, ", ");
                                text!(")");
                            }
                        },
                        K::Square(xs) => {
                            text!("[");
                            sequence!(xs, Expr);
                            text!("]");
                        }
                        K::Bits(xs) => {
                            text!("<<");
                            seq!(xs, Segment, ", ");
                            text!(">>");
                        }
                        K::Curly(xs) if xs.is_empty() || xs.first().is_some_and(is_clause) => {
                            text!("{ ");
                            lines!(xs, Clause);
                            text!(" }");
                        }
                        K::Curly(_) => node!(Block, d),
                        _ => return Err(bad(d, "an expression")),
                    }
                }
            }
            P::Target | P::Operand => {
                let wrap = if matches!(p, P::Target) {
                    matches!(d.kind, K::Int(_) | K::Float(_))
                        || is_match(d)
                        || matches!(round(d),Some([h,_,_]) if is_binop(h))
                } else {
                    is_match(d) || matches!(round(d),Some([h,_,_]) if is_binop(h))
                };
                if wrap {
                    text!("{ ");
                }
                node!(Expr, d);
                if wrap {
                    text!(" }");
                }
            }
            P::Segment | P::PatternSegment => {
                if let Some([h, v, f]) = round(d)
                    && symbol(h, ":")
                {
                    if matches!(p, P::Segment) {
                        node!(Expr, v);
                    } else {
                        node!(Pattern, v);
                    }
                    text!(": ");
                    node!(Format, f);
                } else if matches!(p, P::Segment) {
                    node!(Expr, d);
                } else {
                    node!(Pattern, d);
                }
            }
            P::Format => {
                if let Some([h, v]) = round(d) {
                    text!(label(h)?);
                    text!("(");
                    node!(Expr, v);
                    text!(")");
                } else {
                    text!(label(d)?);
                }
            }
            P::Parameter => {
                if let Some([h, n, t]) = round(d)
                    && symbol(h, "!")
                {
                    text!(label(n)?);
                    text!(" ! ");
                    node!(Type, t);
                } else {
                    node!(Pattern, d);
                }
            }
            P::Block => {
                let K::Curly(xs) = &d.kind else {
                    return Err(bad(d, "a block"));
                };
                text!("{ ");
                lines!(xs, Statement);
                text!(" }");
            }
            P::Statement => {
                if let Some([h, a, b]) = round(d)
                    && symbol(h, "=")
                {
                    node!(Head, a);
                    text!(" = ");
                    node!(Expr, b);
                } else if let Some([h, a, b]) = round(d)
                    && symbol(h, "!")
                {
                    text!("! ");
                    node!(Type, a);
                    text!(" = ");
                    node!(Expr, b);
                } else {
                    node!(Expr, d);
                }
            }
            P::Clause => {
                let Some([h, p, b]) = round(d) else {
                    return Err(bad(d, "a clause"));
                };
                if !symbol(h, "->") {
                    return Err(bad(d, "a clause"));
                }
                node!(ClauseHead, p);
                text!(" -> ");
                node!(Expr, b);
            }
            P::ClauseHead => {
                let (pattern, guard) = match round(d) {
                    Some([h, p, g]) if symbol(h, "?") => (p, Some(g)),
                    _ => (d, None),
                };
                if let Some([h, ps @ ..]) = round(pattern)
                    && symbol(h, "|")
                    && ps.len() >= 2
                {
                    if ps[..ps.len() - 1].iter().any(ends_in_type_test) {
                        return Err(bad(pattern, "a normal-surface clause head"));
                    }
                    seq!(ps, Pattern, " | ");
                } else {
                    node!(Pattern, pattern);
                }
                if let Some(g) = guard {
                    text!(" ? ");
                    node!(Expr, g);
                }
            }
            P::Pattern => {
                if let Some(s) = atom(d) {
                    text!(s);
                } else {
                    match &d.kind {
                        K::Round(xs) => match xs.as_slice() {
                            [] => return Err(bad(d, "a pattern")),
                            [h, n, t] if symbol(h, ":") => {
                                text!(atom(n).ok_or_else(|| bad(n, "a binder"))?);
                                text!(": ");
                                let wrap = matches!(round(t),Some([h,..]) if symbol(h,","));
                                if wrap {
                                    text!("{ ");
                                }
                                node!(Type, t);
                                if wrap {
                                    text!(" }");
                                }
                            }
                            [h, n, p] if symbol(h, "=") => {
                                text!(label(n)?);
                                text!(" = ");
                                node!(Pattern, p);
                            }
                            [h, a] if symbol(h, "-") || symbol(h, "-.") => {
                                text!(show(h));
                                text!(atom(a).ok_or_else(|| bad(a, "a literal"))?);
                            }
                            [h, a, b] if symbol(h, "<>") => {
                                node!(Pattern, a);
                                text!(" <> ");
                                node!(Pattern, b);
                            }
                            [h, a, b] if symbol(h, ".") => {
                                text!(label(a)?);
                                text!(".");
                                text!(import_name(b)?);
                            }
                            [h, ps @ ..] if symbol(h, ",") && !ps.is_empty() => {
                                text!("(");
                                seq!(ps, Pattern, ", ");
                                text!(")");
                            }
                            [h, ps @ ..] => {
                                node!(Name, h);
                                text!("(");
                                sequence!(ps, Pattern);
                                text!(")");
                            }
                        },
                        K::Square(xs) => {
                            text!("[");
                            sequence!(xs, Pattern);
                            text!("]");
                        }
                        K::Bits(xs) => {
                            text!("<<");
                            seq!(xs, PatternSegment, ", ");
                            text!(">>");
                        }
                        _ => return Err(bad(d, "a pattern")),
                    }
                }
            }
        }
        stack.extend(tasks.into_iter().rev());
    }
    Ok((out, anchors))
}

fn is_record(xs: &[Datum]) -> bool {
    xs.first().is_some_and(|first| {
        symbol(first, "..") || matches!(round(first), Some([h, _, _]) if symbol(h, ":"))
    })
}

#[cfg(test)]
mod record_tests {
    use super::*;
    use crate::lisp::parse;

    fn render_one(input: &str, p: P) -> Result<String> {
        let document = parse(input).unwrap();
        render(&document[0], p)
    }

    #[test]
    fn records_keep_spreads_fields_labels_and_grouping() {
        assert_eq!(render_one("()", P::Expr).unwrap(), "()");
        assert!(render_one("()", P::Pattern).is_err());
        assert_eq!(
            render_one("((: x 1) (: \"y-z\" v))", P::Expr).unwrap(),
            "(x: 1, \"y-z\": v)"
        );
        assert_eq!(
            render_one("(.. base (: x v))", P::Expr).unwrap(),
            "(..base, x: v)"
        );
        assert_eq!(
            render_one("(.. R (- X) (: y T))", P::Type).unwrap(),
            "(..R, -X, y: T)"
        );
        assert_eq!(render_one("{ T }", P::Type).unwrap(), "{ T }");
        assert_eq!(render_one("{ x }", P::Expr).unwrap(), "{ x }");
        assert!(render_one("(.. x (- y))", P::Expr).is_err());
        assert!(render_one("(..)", P::Type).is_err());
    }

    #[test]
    fn deeply_nested_record_fields_render_on_small_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let datum = |kind| Datum {
                    kind,
                    span: crate::span::Span::default(),
                    docs: vec![],
                };
                let mut value = datum(K::Name("v".into()));
                for _ in 0..10_000 {
                    value = datum(K::Round(vec![datum(K::Round(vec![
                        datum(K::Symbol(":".into())),
                        datum(K::Name("x".into())),
                        value,
                    ]))]));
                }
                let expected = format!("{}v{}", "(x: ".repeat(10_000), ")".repeat(10_000));
                assert_eq!(render(&value, P::Expr).unwrap(), expected);
            })
            .unwrap()
            .join()
            .unwrap();
    }
    #[test]
    fn deeply_nested_record_text_parses_and_renders_on_small_stack() {
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let mut input = String::with_capacity(100_001);
                for _ in 0..10_000 {
                    input.push_str("((: x ");
                }
                input.push('v');
                for _ in 0..10_000 {
                    input.push_str("))");
                }
                let expected = format!("{}v{}", "(x: ".repeat(10_000), ")".repeat(10_000));
                assert_eq!(render_one(&input, P::Expr).unwrap(), expected);
            })
            .unwrap()
            .join()
            .unwrap();
    }
}
