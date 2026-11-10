//! Transactional item contributions for one compiler-managed macro run.
use super::{DefinitionAbort, DefinitionContext, encode, get, opaque, text};
use crate::{HostImports, Value};
use anyhow::{Result, bail};
use ilang_syntax::lisp::syntax::{Node, Syntax};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub enum ItemRequest {
    Add {
        collection: Syntax,
        target: Syntax,
        data: Vec<(String, String)>,
        at: Syntax,
    },
    Expose {
        name: Syntax,
    },
}

/// A fresh journal for one item macro invocation. The driver takes it only
/// after the result validates and the run succeeds; otherwise it is discarded.
pub struct ItemContext {
    arguments: Arc<[Syntax]>,
    call_site: Syntax,
    requests: Mutex<Vec<ItemRequest>>,
}
impl ItemContext {
    pub fn new(arguments: Arc<[Syntax]>, call_site: Syntax) -> Self {
        Self {
            arguments,
            call_site,
            requests: Mutex::new(Vec::new()),
        }
    }
    pub fn take_requests(&self) -> Vec<ItemRequest> {
        std::mem::take(&mut *self.requests.lock().expect("item journal lock poisoned"))
    }
    fn owns_location(&self, at: &Syntax) -> bool {
        fn contains(syntax: &Syntax, at: &Syntax) -> bool {
            if syntax.span() == at.span() && syntax.scope().module == at.scope().module {
                return true;
            }
            match syntax.node() {
                Node::Round(xs) | Node::Square(xs) | Node::Curly(xs) | Node::Bits(xs) => {
                    xs.iter().any(|s| contains(s, at))
                }
                Node::Quote(s) | Node::Unquote(s) | Node::Splice(s) => contains(s, at),
                _ => false,
            }
        }
        contains(&self.call_site, at) || self.arguments.iter().any(|s| contains(s, at))
    }
}

#[derive(Clone)]
struct Metadata(Option<Arc<MetadataLink>>);
struct MetadataLink {
    key: String,
    value: String,
    previous: Option<Arc<MetadataLink>>,
}
struct ItemResult(Result<(), (String, Syntax)>);

fn is_value_name(syntax: &Syntax) -> bool {
    match syntax.node() {
        Node::Name(_) => true,
        Node::Round(parts) => matches!(parts.as_slice(), [dot, module, name]
            if matches!(dot.node(), Node::Symbol(s) if s == ".")
            && matches!(module.node(), Node::Name(_)) && matches!(name.node(), Node::Name(_))),
        _ => false,
    }
}
fn result(result: Result<(), (String, Syntax)>) -> Value {
    opaque(ItemResult(result))
}
fn item_error(message: &str, at: &Syntax) -> Value {
    result(Err((message.into(), at.clone())))
}
fn unavailable(run: &Option<Arc<DefinitionContext>>, at: &Syntax) -> anyhow::Error {
    let run = run
        .as_ref()
        .expect("compiler instance has no macro run context");
    *run.abort.lock().expect("definition context lock poisoned") =
        Some(DefinitionAbort::CompileError {
            message: "Items is available only during item expansion".into(),
            at: at.clone(),
        });
    anyhow::anyhow!("Items used outside item expansion")
}

pub(super) fn install(
    imports: &mut HostImports,
    context: Option<Arc<ItemContext>>,
    run: Option<Arc<DefinitionContext>>,
) -> Result<()> {
    const NS: &str = "i:compiler/syntax/v1";
    imports.external(NS, "metadata_new", |args| {
        if !args.is_empty() {
            bail!("metadata_new takes no arguments")
        }
        Ok(opaque(Metadata(None)))
    })?;
    imports.external(NS, "metadata_push", |args| {
        let [list, key, value] = args else {
            bail!("metadata_push expects a list, key and value")
        };
        Ok(opaque(Metadata(Some(Arc::new(MetadataLink {
            key: text(key)?.into(),
            value: text(value)?.into(),
            previous: get::<Metadata>(list)?.0.clone(),
        })))))
    })?;
    let add_context = context.clone();
    let add_run = run.clone();
    imports.external(NS, "items_add", move |args| {
        let [collection, target, data, at] = args else {
            bail!("items_add expects four arguments")
        };
        let collection = get::<Syntax>(collection)?;
        let target = get::<Syntax>(target)?;
        let at = get::<Syntax>(at)?;
        let Some(context) = add_context.as_ref() else {
            return Err(unavailable(&add_run, at));
        };
        if !is_value_name(collection) {
            return Ok(item_error("collection must be a value name", collection));
        }
        if !is_value_name(target) {
            return Ok(item_error("target must be a value name", target));
        }
        if !context.owns_location(at) {
            return Ok(item_error(
                "the location must come from this call or its arguments",
                at,
            ));
        }
        let mut data_out = Vec::new();
        let mut keys = HashSet::new();
        let mut current = get::<Metadata>(data)?.0.as_deref();
        while let Some(link) = current {
            if !keys.insert(link.key.clone()) {
                return Ok(item_error("metadata keys must be unique", at));
            }
            data_out.push((link.key.clone(), link.value.clone()));
            current = link.previous.as_deref();
        }
        data_out.reverse();
        context
            .requests
            .lock()
            .expect("item journal lock poisoned")
            .push(ItemRequest::Add {
                collection: collection.clone(),
                target: target.clone(),
                data: data_out,
                at: at.clone(),
            });
        Ok(result(Ok(())))
    })?;
    imports.external(NS, "items_expose", move |args| {
        let [name] = args else {
            bail!("items_expose expects one argument")
        };
        let name = get::<Syntax>(name)?;
        let Some(context) = context.as_ref() else {
            return Err(unavailable(&run, name));
        };
        if !matches!(name.node(), Node::Name(_)) {
            return Ok(item_error("an export request must be a name", name));
        }
        context
            .requests
            .lock()
            .expect("item journal lock poisoned")
            .push(ItemRequest::Expose { name: name.clone() });
        Ok(result(Ok(())))
    })?;
    imports.external(NS, "items_ok", |args| {
        let [value] = args else {
            bail!("items_ok expects one argument")
        };
        Ok(Value::Bool(get::<ItemResult>(value)?.0.is_ok()))
    })?;
    imports.external(NS, "items_message", |args| {
        let [value] = args else {
            bail!("items_message expects one argument")
        };
        Ok(Value::String(
            get::<ItemResult>(value)?
                .0
                .as_ref()
                .expect_err("item result must be an error")
                .0
                .clone(),
        ))
    })?;
    imports.external(NS, "items_at", |args| {
        let [value] = args else {
            bail!("items_at expects one argument")
        };
        Ok(encode(
            get::<ItemResult>(value)?
                .0
                .as_ref()
                .expect_err("item result must be an error")
                .1
                .clone(),
        ))
    })?;
    Ok(())
}

/// Flip the run mark on an entire successful journal, preserving sharing
/// between syntax values in different requests.
pub fn flip_requests(requests: &[ItemRequest], mark: u64) -> Vec<ItemRequest> {
    use ilang_syntax::lisp::syntax::flip_mark_all;
    let mut roots = Vec::new();
    for request in requests {
        match request {
            ItemRequest::Add {
                collection,
                target,
                at,
                ..
            } => roots.extend([collection.clone(), target.clone(), at.clone()]),
            ItemRequest::Expose { name } => roots.push(name.clone()),
        }
    }
    let mut flipped = flip_mark_all(&roots, mark).into_iter();
    requests
        .iter()
        .map(|request| match request {
            ItemRequest::Add { data, .. } => ItemRequest::Add {
                collection: flipped.next().expect("collection root exists"),
                target: flipped.next().expect("target root exists"),
                data: data.clone(),
                at: flipped.next().expect("location root exists"),
            },
            ItemRequest::Expose { .. } => ItemRequest::Expose {
                name: flipped.next().expect("export root exists"),
            },
        })
        .collect()
}
