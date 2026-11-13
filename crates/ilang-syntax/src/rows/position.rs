//! One datum read for one position of `i/syntax` (docs/design/syntax-3.md
//! §6.7), in the rows of §6.8. These are the readers of the parent module,
//! given the names of the positions; `Syntax::show_as` prints their results.

use super::*;

/// The part of a function or of a function type between the parameter part
/// and the body: the result, with the need or the service pattern.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultPart {
    /// `None` for `_`: the function has no written result.
    pub ret: Option<Type>,
    pub needs: Option<Needs>,
}

/// What stands after `<-`.
#[derive(Clone, Debug, PartialEq)]
pub enum Needs {
    /// The service pattern of a function: `(, (: db Db) .. (: env N))`.
    Services(Vec<SItem>),
    /// The need of a function type: `(, Db Log)`, `N`, `(, .. N Db)`.
    Type(Type),
}

/// A datum in the position `Pattern`: a pattern, or the parameter part of a
/// function, which alone may have alternatives in its items and a guard.
#[derive(Clone, Debug, PartialEq)]
pub enum PatternPart {
    Pattern(Pattern),
    Params(Vec<PItem>, Option<Expr>),
}

/// A datum in the position `PatternItem`.
#[derive(Clone, Debug, PartialEq)]
pub enum PatternItem {
    /// `p`, `(= key p…)`, `(| p1 p2 …)`.
    Item(PItem),
    /// `..` alone. `.. r` is two datums, so it is not one item here.
    Rest,
}

/// A datum in the position `Segment`: its value is an expression or a
/// pattern, and the datum does not say which.
#[derive(Clone, Debug, PartialEq)]
pub enum SegmentPart {
    Expr(Segment<Expr>),
    Pattern(Segment<Pattern>),
}

/// `(= …)`, `(: x T)`, a macro call, or one of these in
/// `(@ attribute Item)`.
pub(crate) fn item(d: &Datum) -> R<Item> {
    super::item(d)
}

pub(crate) use super::{expr_from_datum, type_from_datum};

pub(crate) fn import_item(d: &Datum) -> R<Import> {
    import(d)
}

pub(crate) fn export_item(d: &Datum) -> R<Export> {
    export(d)
}

/// `name`, `(. module name)`, or one of them as the head of a Round with
/// the items of the attribute's data.
pub(crate) fn attribute(d: &Datum) -> R<Attribute> {
    let (module, name, args) = match member(d) {
        Some((module, name)) => (module, name, None),
        None => match &d.kind {
            K::Round(parts) if !parts.is_empty() => {
                let Some((module, name)) = member(&parts[0]) else {
                    return err(d, "expected the name of an attribute");
                };
                (module, name, Some(ditems_of(&parts[1..])?))
            }
            _ => return err(d, "expected an attribute"),
        },
    };
    Ok(Attribute {
        module,
        name,
        args,
        span: Sp(d.span),
    })
}

fn one<X>(d: &Datum, mut items: Vec<X>, what: &str) -> R<X> {
    match (items.pop(), items.is_empty()) {
        (Some(item), true) => Ok(item),
        _ => err(d, format!("expected one {what}")),
    }
}

/// `T` or `(= label T)`.
pub(crate) fn type_item(d: &Datum) -> R<TItem> {
    one(
        d,
        titems_of(std::slice::from_ref(d), true)?,
        "item of a data type",
    )
}

/// `e` or `(= label e)`.
pub(crate) fn value_item(d: &Datum) -> R<DItem> {
    one(d, ditems_of(std::slice::from_ref(d))?, "item of data")
}

pub(crate) fn statement(d: &Datum) -> R<Stmt> {
    stmt_of(d)
}

/// The items of a service pattern, when every part is `(: name T)` or the
/// `..` before the last of them.
fn services_of(parts: &[Datum]) -> Option<R<Vec<SItem>>> {
    let service = |p: &Datum| matches!(round(p, ":"), Some([b, _]) if matches!(b.kind, K::Name(_)));
    if parts.is_empty() || !parts.iter().all(|p| symbol(p) == Some("..") || service(p)) {
        return None;
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let rest = symbol(&parts[i]) == Some("..");
        if rest {
            i += 1;
        }
        let Some([b, t]) = parts.get(i).and_then(|p| round(p, ":")) else {
            return Some(err(&parts[i - 1], "`..` needs `(: name N)` after it"));
        };
        out.push(match (lower(b), type_of(t)) {
            (Ok(name), Ok(ty)) => SItem { rest, name, ty },
            (Err(e), _) | (_, Err(e)) => return Some(Err(e)),
        });
        i += 1;
    }
    Some(Ok(out))
}

/// `R`, `_`, or `(<- R S)` where `S` is a need or a service pattern.
pub(crate) fn function_result(d: &Datum) -> R<ResultPart> {
    let (ret, needs) = match round(d, "<-") {
        Some([r, s]) => {
            let needs = match round(s, ",") {
                Some(items) => match services_of(items) {
                    Some(services) => Needs::Services(services?),
                    None => Needs::Type(Type::new(TypeKind::Data(titems_of(items, true)?), s.span)),
                },
                None => Needs::Type(type_of(s)?),
            };
            (r, Some(needs))
        }
        Some(_) => return err(d, "`(<- R S)` has two parts"),
        None => (d, None),
    };
    let ret = match &ret.kind {
        K::Discard(s) if s == "_" => None,
        _ => Some(type_of(ret)?),
    };
    Ok(ResultPart { ret, needs })
}

fn params_of(items: &[Datum]) -> R<Vec<PItem>> {
    items.iter().map(pitem_of).collect()
}

/// A pattern; the parameter part `(, …)` of a function when its items have
/// alternatives; or the guard form `(? (, …) g)`.
pub(crate) fn pattern_part(d: &Datum) -> R<PatternPart> {
    if let Some(parts) = round(d, "?") {
        let [params, guard] = parts else {
            return err(d, "a guard `(? (, …) g)` has two parts");
        };
        let Some(items) = round(params, ",") else {
            return err(
                params,
                "the parameters of a function are a data pattern `(, …)`",
            );
        };
        if let Some(rest) = items.iter().find(|i| symbol(i) == Some("..")) {
            return err(rest, "a parameter group has no rest `..` at its top");
        }
        return Ok(PatternPart::Params(
            params_of(items)?,
            Some(expr_of(guard)?),
        ));
    }
    match pattern_of(d) {
        Ok(p) => Ok(PatternPart::Pattern(p)),
        Err(e) => match round(d, ",") {
            Some(items) if items.iter().all(|i| symbol(i) != Some("..")) => {
                match params_of(items) {
                    Ok(params) => Ok(PatternPart::Params(params, None)),
                    Err(_) => Err(e),
                }
            }
            _ => Err(e),
        },
    }
}

/// `p`, `(= key p…)`, `(| p1 p2 …)`, or `..`.
pub(crate) fn pattern_item(d: &Datum) -> R<PatternItem> {
    if symbol(d) == Some("..") {
        return Ok(PatternItem::Rest);
    }
    pitem_of(d).map(PatternItem::Item)
}

/// `v` or `(: v format)`, where `v` is an expression or a pattern.
pub(crate) fn segment(d: &Datum) -> R<SegmentPart> {
    match segment_of(d, expr_of) {
        Ok(s) => Ok(SegmentPart::Expr(s)),
        Err(e) => match segment_of(d, pattern_of) {
            Ok(s) => Ok(SegmentPart::Pattern(s)),
            Err(_) => Err(e),
        },
    }
}

/// `f` or `(f e)`.
pub(crate) fn format(d: &Datum) -> R<Format> {
    match &d.kind {
        K::Name(_) => Ok(Format {
            name: lower(d)?,
            arg: None,
        }),
        K::Round(parts) => match parts.as_slice() {
            [n, arg] => Ok(Format {
                name: lower(n)?,
                arg: Some(Box::new(expr_of(arg)?)),
            }),
            _ => err(d, "a format is `f` or `(f e)`"),
        },
        _ => err(d, "a format is `f` or `(f e)`"),
    }
}
