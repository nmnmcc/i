//! Position-directed conversion of datums to the normal surface.
use super::{Datum, DatumKind as K, Document, show};
use crate::ast::{Block, Expr, ExprKind, Stmt};
use crate::span::Span;
use crate::walk::{Visitor, Walk};
use crate::{Code, Diagnostic};

type Result<T> = std::result::Result<T, Diagnostic>;

pub(super) fn show_as(d: &Datum, position: super::syntax::Position) -> Result<String> {
    use super::syntax::Position::*;
    match position {
        ImportItem => import(d),
        ExportItem => export(d),
        Item => match &d.kind {
            K::Curly(xs) => joined(xs, item, "\n"),
            _ => item(d),
        },
        Attribute => attribute(d).map(|s| format!("@{s}")),
        Constructor => constructor(d),
        Field => field_type(d),
        Type => typ(d),
        FunctionResult => return_type(d),
        Parameter => parameter(d),
        Expression => checked_expression(d),
        Statement => statement(d),
        CallArgument => argument(d),
        Clause => clause(d),
        ClauseHead => clause_head(d),
        Pattern => pattern(d),
        Segment => expression_segment(d),
        Format => format_spec(d),
    }
}
fn checked_expression(d: &Datum) -> Result<String> {
    let expected = super::to_ast::parse_expression(d)?;
    let text = expr(d)?;
    let (parsed, errors) = crate::parse_module(&format!("value = {text}\n"));
    if !errors.is_empty() {
        return Err(bad(d, "a normal-surface expression"));
    }
    let mut wanted = parsed.clone();
    let Some(crate::ast::Item {
        kind: crate::ast::ItemKind::Binding(binding),
        ..
    }) = wanted.items.first_mut()
    else {
        return Err(bad(d, "a normal-surface expression"));
    };
    binding.value = expected;
    if crate::shape::canon(&wanted) != crate::shape::canon(&parsed) {
        return Err(bad(d, "a normal-surface expression"));
    }
    Ok(text)
}
fn bad(d: &Datum, position: &str) -> Diagnostic {
    Diagnostic::new(
        Code::Unexpected,
        d.span,
        format!("datum is not valid as {position}"),
    )
}
fn symbol(d: &Datum, s: &str) -> bool {
    matches!(&d.kind, K::Symbol(x) if x == s)
}
fn round(d: &Datum) -> Option<&[Datum]> {
    match &d.kind {
        K::Round(xs) => Some(xs),
        _ => None,
    }
}
fn atom(d: &Datum) -> Option<String> {
    match &d.kind {
        K::Name(_) | K::Discard(_) | K::UpName(_) | K::Int(_) | K::Float(_) | K::String(_) => {
            Some(show(d))
        }
        _ => None,
    }
}
fn label(d: &Datum) -> Result<String> {
    match &d.kind {
        K::Name(s) => Ok(s.clone()),
        _ => Err(bad(d, "a label")),
    }
}
fn joined(xs: &[Datum], f: fn(&Datum) -> Result<String>, sep: &str) -> Result<String> {
    Ok(xs.iter().map(f).collect::<Result<Vec<_>>>()?.join(sep))
}

/// Convert a Lisp-surface module to normal source. Errors refer to the datum's
/// original span. A future macro phase consumes quotes without printing them.
pub fn normal_source(document: &Document) -> Result<String> {
    let mut lines = Vec::new();
    let mut headers = 0;
    let mut items_started = false;
    for datum in &document.datums {
        for doc in &datum.docs {
            lines.push(format!(
                "{}{}",
                if doc.module { "////" } else { "///" },
                doc.text
            ));
        }
        if let Some([first, rest @ ..]) = round(datum)
            && symbol(first, ",")
        {
            if items_started || rest.is_empty() || headers == 2 {
                return Err(bad(datum, "a module header"));
            }
            let imports = rest.iter().any(is_import);
            if imports && headers != 0 {
                return Err(bad(datum, "an imports header"));
            }
            headers = if imports { 1 } else { 2 };
            lines.push(format!(
                "({})",
                joined(rest, if imports { import } else { export }, ", ")?
            ));
            continue;
        }
        items_started = true;
        lines.push(item(datum)?);
    }
    for doc in &document.trailing_docs {
        lines.push(format!(
            "{}{}",
            if doc.module { "////" } else { "///" },
            doc.text
        ));
    }
    Ok(if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    })
}

/// Convert to normal source and transfer standalone comments at item and
/// statement boundaries. Return the spans of comments that cannot be kept.
pub fn normal_with_comments(source: &str, document: &Document) -> Result<(String, Vec<Span>)> {
    let output = normal_source(document)?;
    let mut anchors = Vec::new();
    for datum in &document.datums {
        let mut item = datum.clone();
        item.docs.clear();
        let only = Document {
            datums: vec![item],
            trailing_docs: Vec::new(),
        };
        anchors.push((datum.span, normal_source(&only)?.trim_end().to_string()));
    }
    struct Statements(Vec<Span>);
    impl Visitor for Statements {
        fn expr(&mut self, e: &mut Expr, enter: bool) {
            if !enter {
                return;
            }
            let body: Option<&Block> = match &e.kind {
                ExprKind::Block(b) => Some(b),
                ExprKind::Fn(f) => Some(&f.body),
                _ => None,
            };
            if let Some(body) = body {
                for s in &body.stmts {
                    self.0.push(match s {
                        Stmt::Bind(b) => b.span,
                        Stmt::Env { span, .. } => *span,
                        Stmt::Expr(e) => e.span,
                    });
                }
            }
        }
    }
    let mut module = super::to_ast::parse_module(document)?;
    let mut stmts = Statements(Vec::new());
    module.walk(&mut stmts);
    for span in stmts.0 {
        if let Some(d) = document.datums.iter().find_map(|d| find_span(d, span)) {
            anchors.push((span, statement(d)?));
        }
    }
    Ok(super::comments::transfer(
        source, output, anchors, true, false, false,
    ))
}
fn find_span(d: &Datum, span: Span) -> Option<&Datum> {
    if d.span == span {
        return Some(d);
    }
    let children = match &d.kind {
        K::Round(xs) | K::Square(xs) | K::Curly(xs) | K::Bits(xs) => xs.as_slice(),
        K::Quote(x) | K::Unquote(x) | K::Splice(x) => std::slice::from_ref(x.as_ref()),
        _ => &[],
    };
    children.iter().find_map(|child| find_span(child, span))
}
fn is_import(d: &Datum) -> bool {
    round(d).is_some_and(|xs| xs.iter().any(|x| symbol(x, "/") || is_import(x)))
}
fn import(d: &Datum) -> Result<String> {
    let xs = round(d).ok_or_else(|| bad(d, "an import"))?;
    match xs {
        [h, rest @ ..] if symbol(h, "/") && rest.len() >= 2 => joined(rest, label, "/"),
        [h, left, right] if symbol(h, "=") => {
            Ok(format!("{} = {}", import_part(left)?, import(right)?))
        }
        [h, alias, names, path] if symbol(h, "=") => Ok(format!(
            "{} = {} = {}",
            label(alias)?,
            import_part(names)?,
            import(path)?
        )),
        _ => Err(bad(d, "an import")),
    }
}
fn import_part(d: &Datum) -> Result<String> {
    if let Some([h, rest @ ..]) = round(d)
        && symbol(h, ",")
        && !rest.is_empty()
    {
        return Ok(format!("({})", joined(rest, import_name, ", ")?));
    }
    label(d)
}
fn import_name(d: &Datum) -> Result<String> {
    match &d.kind {
        K::Name(s) | K::UpName(s) => Ok(s.clone()),
        _ => Err(bad(d, "an imported name")),
    }
}
fn export(d: &Datum) -> Result<String> {
    if let Some([n, spread]) = round(d)
        && symbol(spread, "..")
    {
        return Ok(format!("{}(..)", import_name(n)?));
    }
    import_name(d)
}
fn item(d: &Datum) -> Result<String> {
    let xs = round(d).ok_or_else(|| bad(d, "an item"))?;
    match xs {
        [h, attr, it] if symbol(h, "@") => Ok(format!("@{}\n{}", attribute(attr)?, item(it)?)),
        [h, left, right] if symbol(h, "=") => Ok(format!(
            "{} = {}",
            item_head(left)?,
            if is_upper_head(left) {
                typ(right)?
            } else {
                expr(right)?
            }
        )),
        [h, left, ty] if symbol(h, ":") => Ok(format!("{}: {}", item_head(left)?, typ(ty)?)),
        [head, body] if matches!(&body.kind, K::Curly(_)) && is_upper_head(head) => {
            let K::Curly(cs) = &body.kind else {
                unreachable!()
            };
            Ok(format!(
                "{} {{ {} }}",
                item_head(head)?,
                joined(cs, constructor, " ")?
            ))
        }
        [head, args @ ..] => Ok(format!(
            "{}({})",
            callee_name(head)?,
            joined(args, argument, ", ")?
        )),
        _ => Err(bad(d, "an item")),
    }
}
fn is_upper_head(d: &Datum) -> bool {
    matches!(d.kind, K::UpName(_)) || round(d).is_some_and(|x| x.first().is_some_and(is_upper_head))
}
fn item_head(d: &Datum) -> Result<String> {
    if let Some([h, left, ty]) = round(d)
        && symbol(h, ":")
    {
        return Ok(format!("{}: {}", item_head(left)?, typ(ty)?));
    }
    if let Some([h, args @ ..]) = round(d)
        && matches!(h.kind, K::Name(_) | K::UpName(_))
        && !args.is_empty()
    {
        return Ok(format!("{}({})", show(h), joined(args, typ, ", ")?));
    }
    pattern(d)
}
fn constructor(d: &Datum) -> Result<String> {
    match &d.kind {
        K::UpName(s) => Ok(s.clone()),
        K::Round(xs) if xs.len() > 1 && matches!(xs[0].kind, K::UpName(_)) => Ok(format!(
            "{}({})",
            show(&xs[0]),
            joined(&xs[1..], field_type, ", ")?
        )),
        _ => Err(bad(d, "a constructor")),
    }
}
fn field_type(d: &Datum) -> Result<String> {
    if let Some([h, n, t]) = round(d)
        && symbol(h, ":")
    {
        return Ok(format!("{}: {}", label(n)?, typ(t)?));
    }
    typ(d)
}
fn attribute(d: &Datum) -> Result<String> {
    if let Some([h, args @ ..]) = round(d)
        && !symbol(h, ".")
    {
        return Ok(format!(
            "{}({})",
            callee_name(h)?,
            joined(args, argument, ", ")?
        ));
    }
    callee_name(d)
}
fn callee_name(d: &Datum) -> Result<String> {
    match &d.kind {
        K::Name(s) | K::UpName(s) => Ok(s.clone()),
        K::Round(xs) if matches!(xs.as_slice(), [h, _, _] if symbol(h, ".")) => {
            Ok(format!("{}.{}", label(&xs[1])?, import_name(&xs[2])?))
        }
        _ => Err(bad(d, "a name")),
    }
}
fn typ(d: &Datum) -> Result<String> {
    if let Some(s) = atom(d) {
        return Ok(s);
    }
    if let Some(xs) = round(d) {
        match xs {
            [h, a, b] if symbol(h, ".") => return Ok(format!("{}.{}", label(a)?, import_name(b)?)),
            [h, a] if symbol(h, "-") && matches!(a.kind, K::Int(_)) => {
                return Ok(format!("-{}", show(a)));
            }
            [h, rest @ ..] if symbol(h, ",") && !rest.is_empty() => {
                return Ok(format!("({})", joined(rest, typ, ", ")?));
            }
            [h, rest @ ..]
                if (symbol(h, "|") || symbol(h, "&") || symbol(h, "-")) && rest.len() >= 2 =>
            {
                return Ok(format!(
                    "{{ {} }}",
                    joined(rest, typ, &format!(" {} ", show(h)))?
                ));
            }
            [h, params, result] if symbol(h, "->") => {
                let ps = round(params).ok_or_else(|| bad(params, "function type parameters"))?;
                return Ok(format!(
                    "{{ ({}) -> {} }}",
                    joined(ps, field_type, ", ")?,
                    return_type(result)?
                ));
            }
            [head]
                if matches!(head.kind, K::Name(_))
                    || matches!(round(head), Some([h, _, n]) if symbol(h, ".") && matches!(n.kind, K::Name(_))) =>
            {
                return Ok(format!("{}()", callee_name(head)?));
            }
            [head, args @ ..] if !args.is_empty() => {
                return Ok(format!(
                    "{}({})",
                    callee_name(head)?,
                    joined(args, typ, ", ")?
                ));
            }
            _ => {}
        }
    }
    if let K::Curly(xs) = &d.kind
        && xs.len() == 1
    {
        return Ok(format!("{{ {} }}", typ(&xs[0])?));
    }
    Err(bad(d, "a type"))
}
fn return_type(d: &Datum) -> Result<String> {
    if let Some([h, r, e]) = round(d) {
        if symbol(h, "^") {
            if matches!(&r.kind, K::Discard(s) if s == "_")
                && matches!(&e.kind, K::UpName(s) if s == "Never")
            {
                return Ok("_".into());
            }
            return Ok(format!("{} ^ {}", typ(r)?, typ(e)?));
        }
        if symbol(h, "!") {
            return Ok(format!("{} ! {}", return_type(r)?, typ(e)?));
        }
    }
    typ(d)
}
fn argument(d: &Datum) -> Result<String> {
    if let Some([h, n, value]) = round(d)
        && symbol(h, ":")
    {
        return Ok(format!("{}: {}", label(n)?, expr(value)?));
    }
    expr(d)
}
fn expr(d: &Datum) -> Result<String> {
    if let Some(s) = atom(d) {
        return Ok(s);
    }
    match &d.kind {
        K::Round(xs) => match xs.as_slice() {
            [] => Ok("()".into()),
            [h, rest @ ..] if symbol(h, ",") && !rest.is_empty() => {
                Ok(format!("({})", joined(rest, expr, ", ")?))
            }
            [h, a, b] if symbol(h, ".") => {
                if let K::Curly(cs) = &b.kind {
                    Ok(format!("{}.{}", field_target(a)?, clauses(cs)?))
                } else {
                    Ok(format!(
                        "{}.{}",
                        field_target(a)?,
                        atom(b).ok_or_else(|| bad(b, "a field"))?
                    ))
                }
            }
            [h, params, result, body] if symbol(h, "->") => {
                let ps = round(params).ok_or_else(|| bad(params, "function parameters"))?;
                let ret = if matches!(&result.kind, K::Discard(s) if s == "_") {
                    String::new()
                } else {
                    format!(" -> {}", return_type(result)?)
                };
                Ok(format!(
                    "({}){} {}",
                    joined(ps, parameter, ", ")?,
                    ret,
                    if matches!(body.kind, K::Curly(_)) {
                        block(body)?
                    } else {
                        format!("{{ {} }}", expr(body)?)
                    }
                ))
            }
            [h, a] if symbol(h, "-") || symbol(h, "-.") => {
                let inner = expr(a)?;
                Ok(format!(
                    "{}{}",
                    show(h),
                    if inner.starts_with('-') || is_match(a) {
                        format!("{{ {inner} }}")
                    } else {
                        inner
                    }
                ))
            }
            [h, a, b] if is_binop(h) => Ok(format!(
                "{} {} {}",
                binary_operand(a)?,
                show(h),
                binary_operand(b)?
            )),
            [head, spread, base, fields @ ..] if symbol(spread, "..") => Ok(format!(
                "{}(..{}{})",
                field_target(head)?,
                expr(base)?,
                fields
                    .iter()
                    .map(|f| argument(f).map(|x| format!(", {x}")))
                    .collect::<Result<String>>()?
            )),
            [head, args @ ..] => Ok(format!(
                "{}({})",
                field_target(head)?,
                joined(args, argument, ", ")?
            )),
        },
        K::Square(xs) => Ok(format!("[{}]", sequence(xs, expr)?)),
        K::Bits(xs) => Ok(format!("<<{}>>", joined(xs, expression_segment, ", ")?)),
        K::Curly(xs) if xs.is_empty() || xs.first().is_some_and(is_clause) => clauses(xs),
        K::Curly(_) => block(d),
        _ => Err(bad(d, "an expression")),
    }
}
fn field_target(d: &Datum) -> Result<String> {
    let s = expr(d)?;
    if matches!(d.kind, K::Int(_) | K::Float(_))
        || is_match(d)
        || matches!(round(d), Some([h, _, _]) if is_binop(h))
    {
        Ok(format!("{{ {s} }}"))
    } else {
        Ok(s)
    }
}
fn is_match(d: &Datum) -> bool {
    matches!(round(d), Some([h, _, b]) if symbol(h, ".") && matches!(b.kind, K::Curly(_)))
}
fn binary_operand(d: &Datum) -> Result<String> {
    let value = expr(d)?;
    if matches!(round(d), Some([h, _, _]) if is_binop(h)) || is_match(d) {
        Ok(format!("{{ {value} }}"))
    } else {
        Ok(value)
    }
}
fn is_binop(d: &Datum) -> bool {
    matches!(&d.kind, K::Symbol(s) if matches!(s.as_str(), "|" | "||" | "&&" | "==" | "!=" | "<" | "<=" | ">" | ">=" | "<." | "<=." | ">." | ">=." | "<>" | "+" | "-" | "+." | "-." | "*" | "/" | "%" | "*." | "/."))
}
fn sequence(xs: &[Datum], f: fn(&Datum) -> Result<String>) -> Result<String> {
    let mut parts = Vec::new();
    let mut i = 0;
    while i < xs.len() {
        if symbol(&xs[i], "..") {
            i += 1;
            if i == xs.len() {
                parts.push("..".into());
                break;
            }
            parts.push(format!("..{}", f(&xs[i])?));
        } else {
            parts.push(f(&xs[i])?);
        }
        i += 1;
    }
    Ok(parts.join(", "))
}
fn expression_segment(d: &Datum) -> Result<String> {
    if let Some([h, val, fmt]) = round(d)
        && symbol(h, ":")
    {
        return Ok(format!("{}: {}", expr(val)?, format_spec(fmt)?));
    }
    expr(d)
}
fn format_spec(d: &Datum) -> Result<String> {
    if let Some([h, v]) = round(d) {
        return Ok(format!("{}({})", label(h)?, expr(v)?));
    }
    label(d)
}
fn parameter(d: &Datum) -> Result<String> {
    if let Some([h, n, t]) = round(d)
        && symbol(h, "!")
    {
        return Ok(format!("{} ! {}", label(n)?, typ(t)?));
    }
    pattern(d)
}
fn block(d: &Datum) -> Result<String> {
    let K::Curly(xs) = &d.kind else {
        return Err(bad(d, "a block"));
    };
    Ok(format!("{{\n{}\n}}", joined(xs, statement, "\n")?))
}
fn statement(d: &Datum) -> Result<String> {
    if let Some([h, a, b]) = round(d) {
        if symbol(h, "=") {
            return Ok(format!("{} = {}", item_head(a)?, expr(b)?));
        }
        if symbol(h, "!") {
            return Ok(format!("! {} = {}", typ(a)?, expr(b)?));
        }
    }
    expr(d)
}
fn is_clause(d: &Datum) -> bool {
    matches!(round(d), Some([h, _, _]) if symbol(h, "->"))
}
fn clauses(cs: &[Datum]) -> Result<String> {
    Ok(format!("{{\n{}\n}}", joined(cs, clause, "\n")?))
}
fn clause(d: &Datum) -> Result<String> {
    let Some([h, p, body]) = round(d) else {
        return Err(bad(d, "a clause"));
    };
    if !symbol(h, "->") {
        return Err(bad(d, "a clause"));
    }
    Ok(format!("{} -> {}", clause_head(p)?, expr(body)?))
}
fn clause_head(p: &Datum) -> Result<String> {
    let (p, guard) = match round(p) {
        Some([h, p, g]) if symbol(h, "?") => (p, Some(g)),
        _ => (p, None),
    };
    let patterns = match round(p) {
        Some([h, ps @ ..]) if symbol(h, "|") && ps.len() >= 2 => {
            if ps[..ps.len() - 1].iter().any(ends_in_type_test) {
                return Err(bad(p, "a normal-surface clause head"));
            }
            joined(ps, pattern, " | ")?
        }
        _ => pattern(p)?,
    };
    Ok(format!(
        "{}{}",
        patterns,
        guard
            .map(|g| expr(g).map(|s| format!(" ? {s}")))
            .transpose()?
            .unwrap_or_default(),
    ))
}
pub(super) fn ends_in_type_test(d: &Datum) -> bool {
    match round(d) {
        Some([h, _, _]) if symbol(h, ":") => true,
        Some([h, _, p]) if symbol(h, "=") => ends_in_type_test(p),
        _ => false,
    }
}
fn pattern(d: &Datum) -> Result<String> {
    if let Some(s) = atom(d) {
        return Ok(s);
    }
    match &d.kind {
        K::Round(xs) => match xs.as_slice() {
            [] => Ok("()".into()),
            [h, n, t] if symbol(h, ":") => Ok(format!(
                "{}: {}",
                atom(n).ok_or_else(|| bad(n, "a binder"))?,
                if matches!(round(t), Some([h, ..]) if symbol(h, ",")) {
                    format!("{{ {} }}", typ(t)?)
                } else {
                    typ(t)?
                }
            )),
            [h, n, p] if symbol(h, "=") => Ok(format!("{} = {}", label(n)?, pattern(p)?)),
            [h, a] if symbol(h, "-") || symbol(h, "-.") => Ok(format!(
                "{}{}",
                show(h),
                atom(a).ok_or_else(|| bad(a, "a literal"))?
            )),
            [h, a, b] if symbol(h, "<>") => Ok(format!("{} <> {}", pattern(a)?, pattern(b)?)),
            [h, a, b] if symbol(h, ".") => Ok(format!("{}.{}", label(a)?, import_name(b)?)),
            [h, ps @ ..] if symbol(h, ",") && !ps.is_empty() => {
                Ok(format!("({})", joined(ps, pattern, ", ")?))
            }
            [h, ps @ ..] => Ok(format!("{}({})", callee_name(h)?, sequence(ps, pattern)?)),
        },
        K::Square(xs) => Ok(format!("[{}]", sequence(xs, pattern)?)),
        K::Bits(xs) => Ok(format!("<<{}>>", joined(xs, pattern_segment, ", ")?)),
        _ => Err(bad(d, "a pattern")),
    }
}
fn pattern_segment(d: &Datum) -> Result<String> {
    if let Some([h, val, fmt]) = round(d)
        && symbol(h, ":")
    {
        return Ok(format!("{}: {}", pattern(val)?, format_spec(fmt)?));
    }
    pattern(d)
}
