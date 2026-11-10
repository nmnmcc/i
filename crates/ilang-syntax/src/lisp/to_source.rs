//! Position-directed conversion of datums to the normal surface.
use super::{Datum, DatumKind as K, Document, show};
use crate::{Code, Diagnostic};

type Result<T> = std::result::Result<T, Diagnostic>;
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
        if let Some([first, rest @ ..]) = round(datum) {
            if symbol(first, ",") {
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
    if let Some([h, rest @ ..]) = round(d) {
        if symbol(h, ",") && !rest.is_empty() {
            return Ok(format!("({})", joined(rest, import_name, ", ")?));
        }
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
    if let Some([n, spread]) = round(d) {
        if symbol(spread, "..") {
            return Ok(format!("{}(..)", import_name(n)?));
        }
    }
    import_name(d)
}
fn item(d: &Datum) -> Result<String> {
    let xs = round(d).ok_or_else(|| bad(d, "an item"))?;
    match xs {
        [h, attr, it] if symbol(h, "@") => Ok(format!("@{} {}", attribute(attr)?, item(it)?)),
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
    if let Some([h, left, ty]) = round(d) {
        if symbol(h, ":") {
            return Ok(format!("{}: {}", item_head(left)?, typ(ty)?));
        }
    }
    if let Some([h, args @ ..]) = round(d) {
        if matches!(h.kind, K::Name(_) | K::UpName(_)) && !args.is_empty() {
            return Ok(format!("{}({})", show(h), joined(args, typ, ", ")?));
        }
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
    if let Some([h, n, t]) = round(d) {
        if symbol(h, ":") {
            return Ok(format!("{}: {}", label(n)?, typ(t)?));
        }
    }
    typ(d)
}
fn attribute(d: &Datum) -> Result<String> {
    if let Some([h, args @ ..]) = round(d) {
        if !args.is_empty() && !symbol(h, ".") {
            return Ok(format!(
                "{}({})",
                callee_name(h)?,
                joined(args, argument, ", ")?
            ));
        }
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
    if let K::Curly(xs) = &d.kind {
        if xs.len() == 1 {
            return Ok(format!("{{ {} }}", typ(&xs[0])?));
        }
    }
    Err(bad(d, "a type"))
}
fn return_type(d: &Datum) -> Result<String> {
    if let Some([h, r, e]) = round(d) {
        if symbol(h, "^") {
            return Ok(format!("{} ? {}", typ(r)?, typ(e)?));
        }
        if symbol(h, "!") {
            return Ok(format!("{} ! {}", return_type(r)?, typ(e)?));
        }
    }
    typ(d)
}
fn argument(d: &Datum) -> Result<String> {
    if let Some([h, n, value]) = round(d) {
        if symbol(h, ":") {
            return Ok(format!("{}: {}", label(n)?, expr(value)?));
        }
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
                    Ok(format!("{} |> {}", field_target(a)?, clauses(cs)?))
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
                    block(body)?
                ))
            }
            [h, a] if symbol(h, "-") || symbol(h, "-.") => Ok(format!("{}{}", show(h), expr(a)?)),
            [h, a, b] if is_binop(h) => Ok(format!(
                "{} {} {}",
                binary_operand(a)?,
                if symbol(h, "|") { "|>".into() } else { show(h) },
                binary_operand(b)?
            )),
            [head, spread, base, fields @ ..] if symbol(spread, "..") => Ok(format!(
                "{}(..{}{})",
                expr(head)?,
                expr(base)?,
                fields
                    .iter()
                    .map(|f| argument(f).map(|x| format!(", {x}")))
                    .collect::<Result<String>>()?
            )),
            [head, args @ ..] => Ok(format!(
                "{}({})",
                expr(head)?,
                joined(args, argument, ", ")?
            )),
            _ => Err(bad(d, "an expression")),
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
        || matches!(round(d), Some([h, _, _]) if is_binop(h))
    {
        Ok(format!("{{ {s} }}"))
    } else {
        Ok(s)
    }
}
fn binary_operand(d: &Datum) -> Result<String> {
    let value = expr(d)?;
    if matches!(round(d), Some([h, _, _]) if is_binop(h)) {
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
    if let Some([h, val, fmt]) = round(d) {
        if symbol(h, ":") {
            return Ok(format!("{}: {}", expr(val)?, format_spec(fmt)?));
        }
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
    if let Some([h, n, t]) = round(d) {
        if symbol(h, "!") {
            return Ok(format!("{} ! {}", label(n)?, typ(t)?));
        }
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
    let (p, guard) = match round(p) {
        Some([h, p, g]) if symbol(h, "?") => (p, Some(g)),
        _ => (p, None),
    };
    let patterns = match round(p) {
        Some([h, ps @ ..]) if symbol(h, "|") && ps.len() >= 2 => joined(ps, pattern, " | ")?,
        _ => pattern(p)?,
    };
    Ok(format!(
        "{}{} -> {}",
        patterns,
        guard
            .map(|g| expr(g).map(|s| format!(" ? {s}")))
            .transpose()?
            .unwrap_or_default(),
        expr(body)?
    ))
}
fn pattern(d: &Datum) -> Result<String> {
    if let Some(s) = atom(d) {
        return Ok(s);
    }
    match &d.kind {
        K::Round(xs) => match xs.as_slice() {
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
            _ => Err(bad(d, "a pattern")),
        },
        K::Square(xs) => Ok(format!("[{}]", sequence(xs, pattern)?)),
        K::Bits(xs) => Ok(format!("<<{}>>", joined(xs, pattern_segment, ", ")?)),
        _ => Err(bad(d, "a pattern")),
    }
}
fn pattern_segment(d: &Datum) -> Result<String> {
    if let Some([h, val, fmt]) = round(d) {
        if symbol(h, ":") {
            return Ok(format!("{}: {}", pattern(val)?, format_spec(fmt)?));
        }
    }
    pattern(d)
}
