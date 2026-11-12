//! Scalar products with explicit shape provenance. Like Heap2Local, require
//! agreement at every definition; materialize only at an observing use.
use super::rewrite;
use crate::ir::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
enum Kind {
    Tuple(usize),
    Nominal(u32, u32, usize),
    Record(Vec<LabelConst>),
}
impl Kind {
    fn width(&self) -> usize {
        match self {
            Self::Tuple(n) | Self::Nominal(_, _, n) => *n,
            Self::Record(labels) => labels.len(),
        }
    }
    fn value(&self, fields: &[Local]) -> Value {
        let fields = fields.iter().copied().map(Atom::Local).collect::<Vec<_>>();
        match self {
            Self::Tuple(_) => Value::Tuple(fields),
            Self::Nominal(ctor, ty, _) => Value::Record {
                ctor: *ctor,
                ty: *ty,
                fields,
            },
            Self::Record(labels) => Value::RecNew(labels.iter().copied().zip(fields).collect()),
        }
    }
}
fn kind(v: &Value, known: &BTreeMap<Local, Kind>) -> Option<Kind> {
    let shape = match v {
        Value::Tuple(fields) => Kind::Tuple(fields.len()),
        Value::Record { ctor, ty, fields } => Kind::Nominal(*ctor, *ty, fields.len()),
        Value::RecNew(fields) => {
            let mut labels = fields.iter().map(|(label, _)| *label).collect::<Vec<_>>();
            labels.sort();
            Kind::Record(labels)
        }
        Value::Atom(Atom::Local(l)) => return known.get(l).cloned(),
        _ => return None,
    };
    (shape.width() <= 4).then_some(shape)
}
#[derive(Clone)]
struct Product {
    kind: Kind,
    fields: Vec<Local>,
}
fn materialize(
    a: &mut Atom,
    products: &BTreeMap<Local, Product>,
    locals: &mut Vec<LocalType>,
    before: &mut Vec<Stmt>,
    cache: &mut BTreeMap<Local, Local>,
) {
    if let Atom::Local(l) = a
        && let Some(product) = products.get(l)
    {
        let temp = *cache.entry(*l).or_insert_with(|| {
            let temp = locals.len() as Local;
            locals.push(LocalType::Value);
            before.push(Stmt::Let {
                dst: temp,
                value: product.kind.value(&product.fields),
                site: None,
            });
            temp
        });
        *a = Atom::Local(temp);
    }
}
pub fn run(f: &mut Func) {
    let mut defs = BTreeMap::<Local, Vec<Value>>::new();
    rewrite::walk(&mut f.body, &mut |s| {
        if let Stmt::Let { dst, value, .. } = s {
            defs.entry(*dst).or_default().push(value.clone());
        }
    });
    let mut shapes = BTreeMap::new();
    loop {
        let before = shapes.len();
        for (&dst, values) in &defs {
            if dst < f.arity || shapes.contains_key(&dst) {
                continue;
            }
            let kinds = values
                .iter()
                .map(|v| kind(v, &shapes))
                .collect::<Option<Vec<_>>>();
            if let Some(kinds) = kinds
                && kinds.iter().all(|k| k == &kinds[0])
            {
                shapes.insert(dst, kinds[0].clone());
            }
        }
        if shapes.len() == before {
            break;
        }
    }
    let products = shapes
        .into_iter()
        .map(|(dst, kind)| {
            let fields = (0..kind.width())
                .map(|_| {
                    let l = f.locals.len() as Local;
                    f.locals.push(LocalType::Value);
                    l
                })
                .collect();
            (dst, Product { kind, fields })
        })
        .collect::<BTreeMap<_, _>>();
    let mut next_label = 0;
    rewrite::walk(&mut f.body, &mut |s| {
        if let Stmt::Block { label, .. } | Stmt::Break(label) = s {
            next_label = next_label.max(*label + 1);
        }
    });
    rewrite::walk(&mut f.body, &mut |s| {
        let mut before = Vec::new();
        let mut cache = BTreeMap::new();
        if let Stmt::Let { dst, value, .. } = s
            && let Some(product) = products.get(dst)
        {
            let mut fields = match value {
                Value::Tuple(fields) | Value::Record { fields, .. } => fields.clone(),
                Value::RecNew(fields) => {
                    let Kind::Record(labels) = &product.kind else {
                        unreachable!()
                    };
                    labels
                        .iter()
                        .map(|label| {
                            fields
                                .iter()
                                .find(|(l, _)| l == label)
                                .expect("record shape label")
                                .1
                                .clone()
                        })
                        .collect()
                }
                Value::Atom(Atom::Local(source)) => products[source]
                    .fields
                    .iter()
                    .copied()
                    .map(Atom::Local)
                    .collect(),
                _ => unreachable!("product definition agrees with its shape"),
            };
            for a in &mut fields {
                materialize(a, &products, &mut f.locals, &mut before, &mut cache);
            }
            before.extend(
                product
                    .fields
                    .iter()
                    .copied()
                    .zip(fields)
                    .map(|(dst, a)| Stmt::Let {
                        dst,
                        value: Value::Atom(a),
                        site: None,
                    }),
            );
            *s = Stmt::Block {
                label: next_label,
                body: before,
            };
            next_label += 1;
            return;
        }
        if let Stmt::Let { value, .. } = s {
            let field = match value {
                Value::Field(Atom::Local(l), index) => products
                    .get(l)
                    .filter(|p| !matches!(p.kind, Kind::Record(_)))
                    .map(|p| p.fields[*index as usize]),
                Value::RecGet(Atom::Local(l), label) => products.get(l).and_then(|p| {
                    if let Kind::Record(labels) = &p.kind {
                        labels.iter().position(|l| l == label).map(|i| p.fields[i])
                    } else {
                        None
                    }
                }),
                _ => None,
            };
            if let Some(field) = field {
                *value = Value::Atom(Atom::Local(field));
            }
        }
        if let Stmt::If { cond, .. } = s {
            fold_cond(cond, &products);
        }
        rewrite::operands(s, &mut |a| {
            materialize(a, &products, &mut f.locals, &mut before, &mut cache)
        });
        if !before.is_empty() {
            before.push(s.clone());
            *s = Stmt::Block {
                label: next_label,
                body: before,
            };
            next_label += 1;
        }
    });
    common(&mut f.body, &mut Vec::new());
}
fn fold_cond(c: &mut Cond, products: &BTreeMap<Local, Product>) {
    match c {
        Cond::Not(c) => fold_cond(c, products),
        Cond::And(a, b) | Cond::Or(a, b) => {
            fold_cond(a, products);
            fold_cond(b, products);
        }
        _ => {
            let result = match c {
                Cond::Tuple(Atom::Local(l), n) => {
                    products.get(l).map(|p| p.kind == Kind::Tuple(*n as usize))
                }
                Cond::Ctor(Atom::Local(l), ctor) => products
                    .get(l)
                    .map(|p| matches!(p.kind, Kind::Nominal(c, _, _) if c == *ctor)),
                Cond::Type(Atom::Local(l), ty) => products
                    .get(l)
                    .map(|p| matches!(p.kind, Kind::Nominal(_, t, _) if t == *ty)),
                Cond::IsRec(Atom::Local(l)) => {
                    products.get(l).map(|p| matches!(p.kind, Kind::Record(_)))
                }
                Cond::Tag(Atom::Local(l), tag) => products.get(l).map(|p| {
                    *tag == match p.kind {
                        Kind::Tuple(_) => 4,
                        Kind::Nominal(..) => 5,
                        Kind::Record(_) => 14,
                    }
                }),
                _ => None,
            };
            if let Some(result) = result {
                *c = Cond::Bool(Atom::Bool(result), true);
            }
        }
    }
}
/// Semantic list/structural-record reads are immutable. A successful read
/// dominates its reuse, including across calls. Closure/env/scratch arrays are
/// never in this cache. Invalidate on every receiver/result redefinition.
fn common(body: &mut [Stmt], cache: &mut Vec<(Value, Local)>) {
    for s in body {
        match s {
            Stmt::Let { dst, value, .. } => {
                cache.retain(|(v, result)| {
                    let mut uses = false;
                    let mut v = v.clone();
                    rewrite::value(&mut v, &mut |a| {
                        if *a == Atom::Local(*dst) {
                            uses = true;
                        }
                    });
                    !uses && result != dst
                });
                if matches!(
                    value,
                    Value::ListHead(_) | Value::ListTail(_) | Value::RecGet(_, _)
                ) {
                    if let Some((_, source)) = cache.iter().find(|(v, _)| v == value) {
                        *value = Value::Atom(Atom::Local(*source));
                    } else {
                        cache.push((value.clone(), *dst));
                    }
                }
            }
            Stmt::If {
                then, otherwise, ..
            } => {
                common(then, &mut cache.clone());
                common(otherwise, &mut cache.clone());
                cache.clear();
            }
            Stmt::Block { body, .. } => {
                common(body, &mut cache.clone());
                cache.clear();
            }
            _ => {}
        }
    }
}
