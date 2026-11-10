//! Compiler syntax services carried over the ordinary opaque-reference ABI.
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

use anyhow::{Context, Result, bail};
use ilang_syntax::lisp::syntax::{Node, Position, Syntax};

use crate::{HostImports, HostReference, Value};

#[derive(Clone)]
struct List(Option<Arc<Link>>);
struct Link {
    value: Syntax,
    previous: Option<Arc<Link>>,
}
struct SyntaxResult(Result<Syntax, String>);
struct TextResult(Result<String, (String, Syntax)>);

fn opaque<T: Send + Sync + 'static>(value: T) -> Value {
    Value::Externref(HostReference::new(value))
}
fn get<T: Send + Sync + 'static>(value: &Value) -> Result<&T> {
    let Value::Externref(reference) = value else {
        bail!("syntax service expected an opaque reference")
    };
    reference
        .downcast_ref::<T>()
        .context("syntax service received the wrong opaque type")
}
/// Give a syntax value to a macro instance through the opaque-reference ABI.
pub fn encode(value: Syntax) -> Value {
    struct Cache {
        references: HashMap<usize, Weak<dyn std::any::Any + Send + Sync>>,
        next_cleanup: usize,
    }
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let mut cache = CACHE
        .get_or_init(|| {
            Mutex::new(Cache {
                references: HashMap::new(),
                next_cleanup: 1024,
            })
        })
        .lock()
        .expect("syntax reference cache lock poisoned");
    let key = value.identity_key();
    if let Some(reference) = cache.references.get(&key).and_then(Weak::upgrade) {
        return Value::Externref(HostReference(reference));
    }
    if cache.references.len() >= cache.next_cleanup {
        cache
            .references
            .retain(|_, reference| reference.strong_count() != 0);
        cache.next_cleanup = cache.references.len().saturating_mul(2).max(1024);
    }
    let reference = HostReference::new(value);
    cache.references.insert(key, Arc::downgrade(&reference.0));
    Value::Externref(reference)
}
/// Read a syntax value returned by a macro instance.
pub fn decode(value: &Value) -> Result<&Syntax> {
    get(value)
}
fn number(value: &Value) -> Result<usize> {
    let Value::Int(n) = value else {
        bail!("syntax service expected Int")
    };
    usize::try_from(n).context("syntax service integer is out of range")
}
fn text(value: &Value) -> Result<&str> {
    let Value::String(s) = value else {
        bail!("syntax service expected String")
    };
    Ok(s)
}
fn result(value: std::result::Result<Syntax, ilang_syntax::lisp::syntax::SyntaxError>) -> Value {
    opaque(SyntaxResult(value.map_err(|e| e.message)))
}
fn position(n: usize) -> Result<Position> {
    Ok(match n {
        0 => Position::ImportItem,
        1 => Position::ExportItem,
        2 => Position::Item,
        3 => Position::Attribute,
        4 => Position::Constructor,
        5 => Position::Field,
        6 => Position::Type,
        7 => Position::FunctionResult,
        8 => Position::Parameter,
        9 => Position::Expression,
        10 => Position::Statement,
        11 => Position::CallArgument,
        12 => Position::Clause,
        13 => Position::ClauseHead,
        14 => Position::Pattern,
        15 => Position::Segment,
        16 => Position::Format,
        _ => bail!("syntax service received an invalid position"),
    })
}
fn child(syntax: &Syntax, index: usize) -> Result<&Syntax> {
    match syntax.node() {
        Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
            xs.get(index).context("syntax child index is out of range")
        }
        Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) if index == 0 => Ok(x),
        _ => bail!("syntax node has no such child"),
    }
}
fn invoke(name: &str, args: &[Value]) -> Result<Value> {
    match (name, args) {
        ("kind", [value]) => Ok(Value::Int(
            (match get::<Syntax>(value)?.node() {
                Node::Name(_) => 0,
                Node::Discard(_) => 1,
                Node::Up(_) => 2,
                Node::Symbol(_) => 3,
                Node::Int(_) => 4,
                Node::Float(_) => 5,
                Node::Text(_) => 6,
                Node::Round(_) => 7,
                Node::Square(_) => 8,
                Node::Curly(_) => 9,
                Node::Bits(_) => 10,
                Node::Quote(_) => 11,
                Node::Unquote(_) => 12,
                Node::Splice(_) => 13,
            })
            .into(),
        )),
        ("text", [value]) => match get::<Syntax>(value)?.node() {
            Node::Name(s) | Node::Discard(s) | Node::Up(s) | Node::Symbol(s) | Node::Text(s) => {
                Ok(Value::String(s.clone()))
            }
            _ => bail!("syntax node does not contain text"),
        },
        ("int", [value]) => match get::<Syntax>(value)?.node() {
            Node::Int(n) => Ok(Value::Int(n.clone().into())),
            _ => bail!("syntax node does not contain Int"),
        },
        ("float", [value]) => match get::<Syntax>(value)?.node() {
            Node::Float(n) => Ok(Value::Float(*n)),
            _ => bail!("syntax node does not contain Float"),
        },
        ("length", [value]) => match get::<Syntax>(value)?.node() {
            Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                Ok(Value::Int(xs.len().into()))
            }
            _ => bail!("syntax node is not a bracket"),
        },
        ("child", [value, index]) => Ok(encode(child(get(value)?, number(index)?)?.clone())),
        ("list_new", []) => Ok(opaque(List(None))),
        ("list_push", [list, value]) => Ok(opaque(List(Some(Arc::new(Link {
            value: get::<Syntax>(value)?.clone(),
            previous: get::<List>(list)?.0.clone(),
        }))))),
        ("make_text", [kind, value, like]) => {
            let value = text(value)?.to_string();
            let node = match number(kind)? {
                0 => Node::Name(value),
                1 => Node::Discard(value),
                2 => Node::Up(value),
                3 => Node::Symbol(value),
                6 => Node::Text(value),
                _ => bail!("invalid text node kind"),
            };
            Ok(result(Syntax::make(node, get(like)?)))
        }
        ("make_int", [Value::Int(value), like]) => {
            let like = get::<Syntax>(like)?;
            let made = match value.to_biguint() {
                Some(value) => Syntax::make(Node::Int(value), like),
                None => Err(ilang_syntax::lisp::syntax::SyntaxError {
                    message: "the node is not a valid syntax atom".into(),
                    span: like.span(),
                }),
            };
            Ok(result(made))
        }
        ("make_float", [Value::Float(value), like]) => {
            Ok(result(Syntax::make(Node::Float(*value), get(like)?)))
        }
        ("make_list", [kind, list, like]) => {
            let mut values = Vec::new();
            let mut link = get::<List>(list)?.0.as_deref();
            while let Some(current) = link {
                values.push(current.value.clone());
                link = current.previous.as_deref();
            }
            values.reverse();
            let node = match number(kind)? {
                7 => Node::Round(values),
                8 => Node::Square(values),
                9 => Node::Curly(values),
                10 => Node::Bits(values),
                _ => bail!("invalid bracket node kind"),
            };
            Ok(result(Syntax::make(node, get(like)?)))
        }
        ("make_one", [kind, value, like]) => {
            let value = get::<Syntax>(value)?.clone();
            let node = match number(kind)? {
                11 => Node::Quote(value),
                12 => Node::Unquote(value),
                13 => Node::Splice(value),
                _ => bail!("invalid quote node kind"),
            };
            Ok(result(Syntax::make(node, get(like)?)))
        }
        ("syntax_ok", [value]) => Ok(Value::Bool(get::<SyntaxResult>(value)?.0.is_ok())),
        ("syntax_value", [value]) => Ok(encode(
            get::<SyntaxResult>(value)?
                .0
                .as_ref()
                .map_err(|_| anyhow::anyhow!("syntax result is an error"))?
                .clone(),
        )),
        ("syntax_error", [value]) => Ok(Value::String(
            get::<SyntaxResult>(value)?
                .0
                .as_ref()
                .err()
                .context("syntax result is not an error")?
                .clone(),
        )),
        ("same_code", [a, b]) => Ok(Value::Bool(get::<Syntax>(a)?.same_code(get(b)?))),
        ("show", [value]) => Ok(Value::String(get::<Syntax>(value)?.show())),
        ("show_as", [value, p]) => {
            let syntax = get::<Syntax>(value)?;
            let result = syntax.show_as(position(number(p)?)?).map_err(|e| {
                let at = syntax
                    .at_span(e.span)
                    .expect("syntax error span belongs to its input")
                    .clone();
                (e.message, at)
            });
            Ok(opaque(TextResult(result)))
        }
        ("text_ok", [value]) => Ok(Value::Bool(get::<TextResult>(value)?.0.is_ok())),
        ("text_value", [value]) => Ok(Value::String(
            get::<TextResult>(value)?
                .0
                .as_ref()
                .map_err(|_| anyhow::anyhow!("text result is an error"))?
                .clone(),
        )),
        ("text_error", [value]) => Ok(Value::String(
            get::<TextResult>(value)?
                .0
                .as_ref()
                .err()
                .context("text result is not an error")?
                .0
                .clone(),
        )),
        ("text_error_at", [value]) => Ok(encode(
            get::<TextResult>(value)?
                .0
                .as_ref()
                .err()
                .context("text result is not an error")?
                .1
                .clone(),
        )),
        _ => bail!("syntax service received an invalid call to {name}"),
    }
}

/// Install only the compiler's syntax operations into an otherwise empty host.
pub fn install(imports: &mut HostImports) -> Result<()> {
    const NAMES: &[&str] = &[
        "kind",
        "text",
        "int",
        "float",
        "length",
        "child",
        "list_new",
        "list_push",
        "make_text",
        "make_int",
        "make_float",
        "make_list",
        "make_one",
        "syntax_ok",
        "syntax_value",
        "syntax_error",
        "same_code",
        "show",
        "show_as",
        "text_ok",
        "text_value",
        "text_error",
        "text_error_at",
    ];
    for &name in NAMES {
        imports.external("i:compiler/syntax/v1", name, move |args| invoke(name, args))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ilang_syntax::lisp::{parse, syntax::Scope};
    use std::collections::BTreeSet;

    #[test]
    fn opaque_values_keep_scopes_and_reject_invalid_atoms() {
        let parsed = parse("(f x)").unwrap();
        let original = Syntax::from_datum(
            &parsed[0],
            &Scope {
                module: 5,
                marks: BTreeSet::from([7]),
            },
        );
        let value = opaque(original.clone());
        assert!(
            matches!(invoke("kind", std::slice::from_ref(&value)).unwrap(), Value::Int(n) if n == 7.into())
        );
        let child = invoke("child", &[value.clone(), Value::Int(1.into())]).unwrap();
        let made = invoke(
            "make_text",
            &[
                Value::Int(0.into()),
                Value::String("name".into()),
                child.clone(),
            ],
        )
        .unwrap();
        assert!(matches!(
            invoke("syntax_ok", std::slice::from_ref(&made)).unwrap(),
            Value::Bool(true)
        ));
        let made = invoke("syntax_value", &[made]).unwrap();
        let made = get::<Syntax>(&made).unwrap();
        assert_eq!(made.show(), "name");
        assert_eq!(made.scope(), get::<Syntax>(&child).unwrap().scope());
        let invalid = invoke(
            "make_text",
            &[
                Value::Int(0.into()),
                Value::String("not valid".into()),
                child,
            ],
        )
        .unwrap();
        assert!(matches!(
            invoke("syntax_ok", &[invalid]).unwrap(),
            Value::Bool(false)
        ));
    }

    #[test]
    fn persistent_builders_preserve_order_and_prior_versions() {
        let a = opaque(Syntax::from_datum(
            &parse("a").unwrap()[0],
            &Scope {
                module: 0,
                marks: BTreeSet::new(),
            },
        ));
        let b = opaque(Syntax::from_datum(
            &parse("b").unwrap()[0],
            &Scope {
                module: 0,
                marks: BTreeSet::new(),
            },
        ));
        let empty = invoke("list_new", &[]).unwrap();
        let one = invoke("list_push", &[empty, a.clone()]).unwrap();
        let two = invoke("list_push", &[one.clone(), b]).unwrap();
        for (list, expected) in [(one, "(a)"), (two, "(a b)")] {
            let made = invoke("make_list", &[Value::Int(7.into()), list, a.clone()]).unwrap();
            let syntax = invoke("syntax_value", &[made]).unwrap();
            assert!(
                matches!(invoke("show", &[syntax]).unwrap(), Value::String(s) if s == expected)
            );
        }
    }

    #[test]
    fn show_as_reports_the_offending_nested_syntax() {
        let parsed = parse("(f (,))").unwrap();
        let syntax = Syntax::from_datum(
            &parsed[0],
            &Scope {
                module: 0,
                marks: BTreeSet::new(),
            },
        );
        let result = invoke("show_as", &[opaque(syntax), Value::Int(9.into())]).unwrap();
        assert!(matches!(
            invoke("text_ok", std::slice::from_ref(&result)).unwrap(),
            Value::Bool(false)
        ));
        let at = invoke("text_error_at", &[result]).unwrap();
        assert!(matches!(invoke("show", &[at]).unwrap(), Value::String(s) if s == "(,)"));
    }
}
