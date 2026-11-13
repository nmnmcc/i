//! Data shapes (syntax-3 §2.3, §6.2): the number of positions and the set
//! of labels of some data. A shape is interned once per process, so that a
//! [`ShapeId`] is a small copyable head part. Parts of a data value or type
//! are kept in canonical order: the positions, then the labels in UTF-8
//! byte order.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// A data shape. A shape with no labels is its number of positions, so
/// every tuple shape is the same id in every program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShapeId(u32);

const LABELLED: u32 = 1 << 31;

#[derive(Default)]
struct Table {
    shapes: Vec<(u32, Arc<[String]>)>,
    ids: HashMap<(u32, Arc<[String]>), u32>,
}

fn table() -> &'static RwLock<Table> {
    static TABLE: OnceLock<RwLock<Table>> = OnceLock::new();
    TABLE.get_or_init(Default::default)
}

impl ShapeId {
    /// The shape of a tuple of `n` parts (`n` may be 0: `()`).
    pub fn positional(n: u32) -> ShapeId {
        assert!(n < LABELLED, "too many positions");
        ShapeId(n)
    }

    /// The shape with `positions` positions and these labels, in any order.
    /// A repeated label is a broken invariant.
    pub fn new(positions: u32, labels: impl IntoIterator<Item = String>) -> ShapeId {
        let mut labels: Vec<String> = labels.into_iter().collect();
        if labels.is_empty() {
            return ShapeId::positional(positions);
        }
        labels.sort();
        let before = labels.len();
        labels.dedup();
        assert_eq!(before, labels.len(), "a label is given twice in a shape");
        let key = (positions, Arc::<[String]>::from(labels));
        if let Some(id) = table().read().unwrap().ids.get(&key) {
            return ShapeId(*id);
        }
        let mut t = table().write().unwrap();
        if let Some(id) = t.ids.get(&key) {
            return ShapeId(*id);
        }
        let id = LABELLED | t.shapes.len() as u32;
        t.shapes.push(key.clone());
        t.ids.insert(key, id);
        ShapeId(id)
    }

    /// The number of positions.
    pub fn positions(self) -> u32 {
        if self.0 & LABELLED == 0 {
            self.0
        } else {
            table().read().unwrap().shapes[(self.0 & !LABELLED) as usize].0
        }
    }

    /// The labels, in UTF-8 byte order.
    pub fn labels(self) -> Arc<[String]> {
        if self.0 & LABELLED == 0 {
            Arc::from(Vec::new())
        } else {
            table().read().unwrap().shapes[(self.0 & !LABELLED) as usize]
                .1
                .clone()
        }
    }

    pub fn has_labels(self) -> bool {
        self.0 & LABELLED != 0
    }

    /// The number of parts.
    pub fn len(self) -> usize {
        self.positions() as usize + self.labels().len()
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// The canonical index of the part with this label.
    pub fn label_index(self, label: &str) -> Option<usize> {
        let labels = self.labels();
        labels
            .binary_search_by(|l| l.as_str().cmp(label))
            .ok()
            .map(|i| self.positions() as usize + i)
    }

    /// The key of each part, in canonical order: `None` for a position.
    pub fn keys(self) -> Vec<Option<String>> {
        let mut keys: Vec<Option<String>> = (0..self.positions()).map(|_| None).collect();
        keys.extend(self.labels().iter().cloned().map(Some));
        keys
    }

    /// The text of a shape with these part texts: `(A, B)`, `(a: A, b: B)`.
    pub fn text(self, parts: &[String]) -> String {
        let keys = self.keys();
        let items: Vec<String> = keys
            .iter()
            .zip(parts)
            .map(|(k, p)| match k {
                Some(l) => format!("{l}: {p}"),
                None => p.clone(),
            })
            .collect();
        format!("({})", items.join(", "))
    }
}
