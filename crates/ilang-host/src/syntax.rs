//! Compiler syntax services carried over the ordinary opaque-reference ABI.
mod items;
pub use items::{ItemContext, ItemRequest, flip_output_and_requests, flip_requests};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use ilang_syntax::lisp::syntax::{Capture, Node, Position, Substitution, Syntax};

use crate::macro_budget::MacroBudget;
use crate::{HostImports, HostReference, Value};

#[derive(Clone)]
struct List(Option<Arc<Link>>);
struct Link {
    value: Syntax,
    previous: Option<Arc<Link>>,
}
struct SyntaxResult(Result<Syntax, String>);
struct TextResult(Result<String, (String, Syntax)>);
struct MatchResult(Option<Vec<Capture>>);
struct DefinitionResult(Result<Syntax, (String, Syntax)>);

/// A compiler definition resolver's answer or control outcome.
#[derive(Clone, Debug)]
pub enum DefinitionReply {
    Answer(Syntax),
    Error { message: String, at: Syntax },
    Deferred { at: Syntax },
    CompileError { message: String, at: Syntax },
}

/// An uncatchable query outcome observed after the macro instance stops.
#[derive(Clone, Debug)]
pub enum DefinitionAbort {
    Deferred { at: Syntax },
    CompileError { message: String, at: Syntax },
    MacroError { message: String, at: Syntax },
}

type DefinitionCallback = dyn Fn(&Syntax) -> DefinitionReply + Send + Sync;

/// The resolver and uncatchable outcome for one fresh macro instance.
pub struct DefinitionContext {
    callback: Arc<DefinitionCallback>,
    abort: Mutex<Option<DefinitionAbort>>,
    notes: Mutex<Vec<String>>,
}

impl DefinitionContext {
    pub fn new(callback: impl Fn(&Syntax) -> DefinitionReply + Send + Sync + 'static) -> Self {
        Self {
            callback: Arc::new(callback),
            abort: Mutex::new(None),
            notes: Mutex::new(Vec::new()),
        }
    }
    /// Take this run's buffered diagnostic notes after a successful commit.
    pub fn take_notes(&self) -> Vec<String> {
        std::mem::take(&mut *self.notes.lock().expect("diagnostic buffer lock poisoned"))
    }
    pub fn take_abort(&self) -> Option<DefinitionAbort> {
        self.abort
            .lock()
            .expect("definition context lock poisoned")
            .take()
    }
}

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
    Value::Externref(HostReference(value.host_identity()))
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
        ("definition_ok", [result]) => Ok(Value::Bool(get::<DefinitionResult>(result)?.0.is_ok())),
        ("definition_value", [result]) => Ok(encode(
            get::<DefinitionResult>(result)?
                .0
                .as_ref()
                .expect("definition result is an error")
                .clone(),
        )),
        ("definition_message", [result]) => Ok(Value::String(
            get::<DefinitionResult>(result)?
                .0
                .as_ref()
                .expect_err("definition result is not an error")
                .0
                .clone(),
        )),
        ("definition_at", [result]) => Ok(encode(
            get::<DefinitionResult>(result)?
                .0
                .as_ref()
                .expect_err("definition result is not an error")
                .1
                .clone(),
        )),
        ("match", [pattern, value]) => {
            let result = get::<Syntax>(pattern)?
                .match_quote(get(value)?)
                .expect("a compiled quote pattern is valid");
            Ok(opaque(MatchResult(result)))
        }
        ("match_ok", [result]) => Ok(Value::Bool(get::<MatchResult>(result)?.0.is_some())),
        ("match_one", [result, index]) => {
            let captures = get::<MatchResult>(result)?
                .0
                .as_ref()
                .expect("cannot read captures from a failed match");
            let capture = &captures[number(index)?];
            let Substitution::One(value) = &capture.value else {
                panic!("quote capture is a splice")
            };
            Ok(encode(value.clone()))
        }
        ("match_many_len", [result, index]) => {
            let captures = get::<MatchResult>(result)?
                .0
                .as_ref()
                .expect("cannot read captures from a failed match");
            let capture = &captures[number(index)?];
            let Substitution::Many(values) = &capture.value else {
                panic!("quote capture is not a splice")
            };
            Ok(Value::Int(values.len().into()))
        }
        ("match_many_child", [result, index, position]) => {
            let captures = get::<MatchResult>(result)?
                .0
                .as_ref()
                .expect("cannot read captures from a failed match");
            let capture = &captures[number(index)?];
            let Substitution::Many(values) = &capture.value else {
                panic!("quote capture is not a splice")
            };
            Ok(encode(values[number(position)?].clone()))
        }
        ("rebuild", [template, list]) => {
            let mut children = Vec::new();
            let mut link = get::<List>(list)?.0.as_deref();
            while let Some(current) = link {
                children.push(current.value.clone());
                link = current.previous.as_deref();
            }
            children.reverse();
            Ok(encode(get::<Syntax>(template)?.replace_children(children)))
        }
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

pub fn charge_tree(budget: &MacroBudget, root: &Syntax, output: bool) -> Result<()> {
    enum Visit<'a> {
        Node(&'a Syntax),
        Leave,
    }
    let mut pending = vec![Visit::Node(root)];
    while let Some(visit) = pending.pop() {
        let Visit::Node(s) = visit else {
            budget.leave_frame();
            continue;
        };
        budget.enter_frame(1)?;
        budget.charge(1, 1)?;
        pending.push(Visit::Leave);
        let bytes = match s.node() {
            Node::Name(x) | Node::Discard(x) | Node::Up(x) | Node::Symbol(x) | Node::Text(x) => {
                x.len() as u64
            }
            Node::Int(x) => x.bits().div_ceil(8),
            Node::Float(_) => 8,
            Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                // Charge before the traversal stack may grow.
                budget.charge(xs.len() as u64, 4 + xs.len() as u64)?;
                pending.extend(xs.iter().rev().map(Visit::Node));
                0
            }
            Node::Quote(x) | Node::Unquote(x) | Node::Splice(x) => {
                budget.charge(1, 1)?;
                pending.push(Visit::Node(x));
                0
            }
        };
        // Escaping one input byte can produce several ASCII characters (for
        // example a control character); bound both construction and output.
        let work = if output {
            bytes.saturating_mul(12)
        } else {
            bytes
        };
        budget.charge(work, if output { 4 + work.div_ceil(8) } else { 0 })?;
    }
    Ok(())
}

/// Preflight a definition answer before its syntax nodes are constructed.
pub fn charge_definition(
    budget: &MacroBudget,
    cost: ilang_syntax::lisp::definition_table::DefinitionCost<'_>,
) -> Result<()> {
    match cost {
        ilang_syntax::lisp::definition_table::DefinitionCost::Syntax(syntax) => {
            charge_tree(budget, syntax, true)
        }
        // All five synthesized builtins fit this fixed bound, including the
        // lexer, answer nodes, parameter identities and temporary strings.
        ilang_syntax::lisp::definition_table::DefinitionCost::Builtin(_) => {
            budget.charge(512, 512)?;
            Ok(())
        }
    }
}

fn charge_call(budget: &MacroBudget, name: &str, args: &[Value]) -> Result<()> {
    // Every compiler service call takes one unit and a bounded result wrapper.
    budget.charge(1, 8)?;
    match (name, args) {
        ("make_text", [_, Value::String(s), _]) => {
            let n = s.len() as u64;
            budget.charge(n, 4 + n.div_ceil(8))?;
        }
        ("make_int", [Value::Int(n), _]) => {
            let bytes = n.bits().div_ceil(8);
            budget.charge(bytes, 4 + bytes.div_ceil(8))?;
        }
        ("make_list", [_, list, _]) | ("rebuild", [_, list]) => {
            let mut link = get::<List>(list)?.0.as_deref();
            while let Some(current) = link {
                budget.charge(1, 1)?;
                link = current.previous.as_deref();
            }
        }
        ("match", [pattern, value]) => {
            charge_tree(budget, get(pattern)?, false)?;
            charge_tree(budget, get(value)?, false)?;
        }
        ("text", [value]) => {
            let syntax = get::<Syntax>(value)?;
            if let Node::Name(s)
            | Node::Discard(s)
            | Node::Up(s)
            | Node::Symbol(s)
            | Node::Text(s) = syntax.node()
            {
                let bytes = s.len() as u64;
                budget.charge(bytes, 4 + bytes.div_ceil(8))?;
            }
        }
        ("int", [value]) => {
            if let Node::Int(n) = get::<Syntax>(value)?.node() {
                let bytes = n.bits().div_ceil(8);
                budget.charge(bytes, 4 + bytes.div_ceil(8))?;
            }
        }
        ("definition_message", [value]) => {
            let message = &get::<DefinitionResult>(value)?
                .0
                .as_ref()
                .expect_err("definition result must be an error")
                .0;
            let bytes = message.len() as u64;
            budget.charge(bytes, 4 + bytes.div_ceil(8))?;
        }
        ("syntax_error", [value]) => {
            let message = get::<SyntaxResult>(value)?
                .0
                .as_ref()
                .expect_err("syntax result must be an error");
            let bytes = message.len() as u64;
            budget.charge(bytes, 4 + bytes.div_ceil(8))?;
        }
        ("text_error", [value]) => {
            let message = &get::<TextResult>(value)?
                .0
                .as_ref()
                .expect_err("text result must be an error")
                .0;
            let bytes = message.len() as u64;
            budget.charge(bytes, 4 + bytes.div_ceil(8))?;
        }
        ("show" | "show_as", [value, ..]) => charge_tree(budget, get(value)?, true)?,
        ("same_code", [left, right]) => {
            charge_tree(budget, get(left)?, false)?;
            charge_tree(budget, get(right)?, false)?;
        }
        _ => {}
    }
    Ok(())
}

/// Install only the compiler's syntax operations into an otherwise empty host.
pub fn install(imports: &mut HostImports) -> Result<()> {
    install_with_templates(imports, Arc::from([]))
}

/// Install syntax operations with the templates belonging to this instance.
pub fn install_with_templates(imports: &mut HostImports, templates: Arc<[Syntax]>) -> Result<()> {
    install_with_context(imports, templates, None)
}

/// Install syntax operations and an optional per-instance definition resolver.
pub fn install_with_context(
    imports: &mut HostImports,
    templates: Arc<[Syntax]>,
    definition: Option<Arc<DefinitionContext>>,
) -> Result<()> {
    install_with_items(imports, templates, definition, None)
}

/// Install compiler syntax operations with a per-run item journal.
pub fn install_with_items(
    imports: &mut HostImports,
    templates: Arc<[Syntax]>,
    definition: Option<Arc<DefinitionContext>>,
    items: Option<Arc<ItemContext>>,
) -> Result<()> {
    install_inner(imports, templates, definition, items, None)
}

/// Install the syntax service with the same invocation budget as Wasm.
pub fn install_with_budget(
    imports: &mut HostImports,
    templates: Arc<[Syntax]>,
    definition: Option<Arc<DefinitionContext>>,
    items: Option<Arc<ItemContext>>,
    budget: MacroBudget,
) -> Result<()> {
    imports.set_macro_budget(budget.clone());
    install_inner(imports, templates, definition, items, Some(budget))
}

/// Install metered syntax services without templates or compiler contexts.
pub fn install_metered(imports: &mut HostImports, budget: MacroBudget) -> Result<()> {
    install_with_budget(imports, Arc::from([]), None, None, budget)
}

fn install_inner(
    imports: &mut HostImports,
    templates: Arc<[Syntax]>,
    definition: Option<Arc<DefinitionContext>>,
    items: Option<Arc<ItemContext>>,
    budget: Option<MacroBudget>,
) -> Result<()> {
    items::install(imports, items, definition.clone(), budget.clone())?;
    const NAMES: &[&str] = &[
        "kind",
        "text",
        "int",
        "float",
        "length",
        "child",
        "list_new",
        "list_push",
        "rebuild",
        "match",
        "match_ok",
        "match_one",
        "match_many_len",
        "match_many_child",
        "definition_ok",
        "definition_value",
        "definition_message",
        "definition_at",
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
        let budget = budget.clone();
        imports.external("i:compiler/syntax/v1", name, move |args| {
            if let Some(budget) = &budget {
                charge_call(budget, name, args)?;
            }
            invoke(name, args)
        })?;
    }
    let note_context = definition.clone();
    let note_budget = budget.clone();
    imports.external("i:compiler/syntax/v1", "note", move |args| {
        let [message] = args else {
            bail!("diagnostic note expects one message")
        };
        if let Some(budget) = &note_budget {
            let bytes = text(message)?.len() as u64;
            budget.charge(1 + bytes, 8 + 4 + bytes.div_ceil(8))?;
        }
        let context = note_context
            .as_ref()
            .expect("compiler instance has no macro run context");
        context
            .notes
            .lock()
            .expect("diagnostic buffer lock poisoned")
            .push(text(message)?.to_owned());
        Ok(Value::Nil)
    })?;
    let report_context = definition.clone();
    let report_budget = budget.clone();
    imports.external("i:compiler/syntax/v1", "report_error", move |args| {
        let [message, at] = args else {
            bail!("macro error reporter expects message and syntax")
        };
        if let Some(budget) = &report_budget {
            let bytes = text(message)?.len() as u64;
            budget.charge(1 + bytes, 8 + 4 + bytes.div_ceil(8))?;
        }
        let context = report_context
            .as_ref()
            .expect("compiler instance has no macro run context");
        *context
            .abort
            .lock()
            .expect("definition context lock poisoned") = Some(DefinitionAbort::MacroError {
            message: text(message)?.to_owned(),
            at: get::<Syntax>(at)?.clone(),
        });
        bail!("macro reported a syntax error")
    })?;
    let definition_budget = budget.clone();
    imports.external("i:compiler/syntax/v1", "definition", move |args| {
        let [value] = args else {
            bail!("definition resolver expects one syntax value")
        };
        if let Some(budget) = &definition_budget {
            budget.charge(1, 8)?;
            charge_tree(budget, get::<Syntax>(value)?, false)?;
        }
        let context = definition
            .as_ref()
            .expect("compiler instance has no definition resolver");
        match (context.callback)(get::<Syntax>(value)?) {
            DefinitionReply::Answer(value) => {
                if let Some(budget) = &definition_budget {
                    charge_tree(budget, &value, true)?;
                }
                Ok(opaque(DefinitionResult(Ok(value))))
            }
            DefinitionReply::Error { message, at } => {
                Ok(opaque(DefinitionResult(Err((message, at)))))
            }
            DefinitionReply::Deferred { at } => {
                *context
                    .abort
                    .lock()
                    .expect("definition context lock poisoned") =
                    Some(DefinitionAbort::Deferred { at });
                bail!("definition query deferred")
            }
            DefinitionReply::CompileError { message, at } => {
                *context
                    .abort
                    .lock()
                    .expect("definition context lock poisoned") =
                    Some(DefinitionAbort::CompileError { message, at });
                bail!("definition query produced a compile error")
            }
        }
    })?;
    let template_budget = budget;
    imports.external("i:compiler/syntax/v1", "template", move |args| {
        let [index] = args else {
            bail!("syntax template expects one index")
        };
        if let Some(budget) = &template_budget {
            budget.charge(1, 8)?;
        }
        let index = number(index)?;
        let syntax = templates
            .get(index)
            .context("syntax template index is out of range")?;
        Ok(encode(syntax.clone()))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macro_budget::{Limit, Limits};
    use ilang_syntax::lisp::{parse, syntax::Scope};
    use std::collections::BTreeSet;

    #[test]
    fn compiler_syntax_traversal_charges_work_memory_and_depth() {
        let parsed = parse("(((alpha beta)))").unwrap();
        let value = opaque(Syntax::from_datum(
            &parsed[0],
            &Scope {
                module: 0,
                marks: BTreeSet::new(),
            },
        ));
        let budget = MacroBudget::new(Limits {
            work: 1000,
            allocation: 1000,
            frames: 100,
        });
        charge_call(&budget, "show", std::slice::from_ref(&value)).unwrap();
        let used = budget.usage();
        assert!(used.work > 10 && used.allocation > 10);
        assert_eq!(used.frames, 0);

        let shallow = MacroBudget::new(Limits {
            work: 1000,
            allocation: 1000,
            frames: 2,
        });
        assert!(charge_call(&shallow, "show", std::slice::from_ref(&value)).is_err());
        assert_eq!(shallow.exceeded(), Some(Limit::Frames));

        let small = MacroBudget::new(Limits {
            work: 1000,
            allocation: 8,
            frames: 100,
        });
        assert!(charge_call(&small, "show", std::slice::from_ref(&value)).is_err());
        assert_eq!(small.exceeded(), Some(Limit::Allocation));
    }

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
