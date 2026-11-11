//! Deterministic accounting inserted before Wasm optimization.
//!
//! Charges are calls to trusted compiler imports; optimizers cannot remove
//! or move them across observable effects. Ordinary target modules use no
//! accounting imports and retain their existing byte representation.

use std::collections::{BTreeMap, HashMap};

const IMPORTS: &str = r#"
(import "ilang:macro/budget/v1" "charge" (func $macro_charge (param i64 i64)))
(import "ilang:macro/budget/v1" "enter" (func $macro_enter (param i32)))
(import "ilang:macro/budget/v1" "leave" (func $macro_leave))
(import "ilang:macro/budget/v1" "restore" (func $macro_restore (param i32)))
"#;

const HELPERS: &str = r#"
;; Return the original length after charging, so the argument is evaluated
;; exactly once and the charge precedes the allocation/copy.
(func $macro_array_words (param $n i32) (result i32)
  (call $macro_charge (i64.extend_i32_u (local.get $n)) (i64.add (i64.const 4) (i64.extend_i32_u (local.get $n))))
  (local.get $n))
(func $macro_array_halves (param $n i32) (result i32)
  (call $macro_charge (i64.extend_i32_u (local.get $n)) (i64.add (i64.const 4)
    (i64.shr_u (i64.add (i64.extend_i32_u (local.get $n)) (i64.const 1)) (i64.const 1))))
  (local.get $n))
(func $macro_array_quarters (param $n i32) (result i32)
  (call $macro_charge (i64.extend_i32_u (local.get $n)) (i64.add (i64.const 4)
    (i64.shr_u (i64.add (i64.extend_i32_u (local.get $n)) (i64.const 3)) (i64.const 2))))
  (local.get $n))
(func $macro_array_bytes (param $n i32) (result i32)
  (call $macro_charge (i64.extend_i32_u (local.get $n)) (i64.add (i64.const 4)
    (i64.shr_u (i64.add (i64.extend_i32_u (local.get $n)) (i64.const 7)) (i64.const 3))))
  (local.get $n))
(func $macro_copy_units (param $n i32) (result i32)
  (call $macro_charge (i64.extend_i32_u (local.get $n)) (i64.const 0))
  (local.get $n))
(func $macro_memory_pages (param $n i32) (result i32)
  (call $macro_charge (i64.mul (i64.extend_i32_u (local.get $n)) (i64.const 65536))
    (i64.mul (i64.extend_i32_u (local.get $n)) (i64.const 8192)))
  (local.get $n))
"#;

#[derive(Debug)]
struct Node {
    start: usize,
    end: usize,
    children: Vec<Node>,
    list: bool,
}

impl Node {
    fn atom<'a>(&self, text: &'a str) -> &'a str {
        assert!(!self.list, "expected WAT atom");
        &text[self.start..self.end]
    }

    fn head<'a>(&self, text: &'a str) -> Option<&'a str> {
        self.children
            .first()
            .filter(|x| !x.list)
            .map(|x| x.atom(text))
    }
}

struct Parser<'a> {
    text: &'a str,
    at: usize,
}

impl Parser<'_> {
    fn skip(&mut self) {
        let b = self.text.as_bytes();
        loop {
            while self.at < b.len() && b[self.at].is_ascii_whitespace() {
                self.at += 1;
            }
            if b.get(self.at..self.at + 2) == Some(b";;") {
                while self.at < b.len() && b[self.at] != b'\n' {
                    self.at += 1;
                }
            } else if b.get(self.at..self.at + 2) == Some(b"(;") {
                self.at += 2;
                let mut depth = 1;
                while depth != 0 {
                    assert!(self.at < b.len(), "unterminated WAT comment");
                    if b.get(self.at..self.at + 2) == Some(b"(;") {
                        depth += 1;
                        self.at += 2;
                    } else if b.get(self.at..self.at + 2) == Some(b";)") {
                        depth -= 1;
                        self.at += 2;
                    } else {
                        self.at += 1;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn node(&mut self) -> Node {
        self.skip();
        let b = self.text.as_bytes();
        let start = self.at;
        assert!(start < b.len(), "expected WAT node");
        if b[start] == b'(' {
            self.at += 1;
            let mut children = Vec::new();
            loop {
                self.skip();
                assert!(self.at < b.len(), "unterminated WAT list");
                if b[self.at] == b')' {
                    self.at += 1;
                    break;
                }
                children.push(self.node());
            }
            Node {
                start,
                end: self.at,
                children,
                list: true,
            }
        } else if b[start] == b'"' {
            self.at += 1;
            loop {
                assert!(self.at < b.len(), "unterminated WAT string");
                match b[self.at] {
                    b'\\' => {
                        self.at += 2;
                    }
                    b'"' => {
                        self.at += 1;
                        break;
                    }
                    _ => self.at += 1,
                }
            }
            Node {
                start,
                end: self.at,
                children: Vec::new(),
                list: false,
            }
        } else {
            while self.at < b.len()
                && !b[self.at].is_ascii_whitespace()
                && b[self.at] != b'('
                && b[self.at] != b')'
            {
                self.at += 1;
            }
            assert!(self.at > start, "invalid WAT atom");
            Node {
                start,
                end: self.at,
                children: Vec::new(),
                list: false,
            }
        }
    }
}

type Inserts = BTreeMap<usize, Vec<(u8, String)>>;
fn insert(edits: &mut Inserts, at: usize, priority: u8, text: impl Into<String>) {
    edits.entry(at).or_default().push((priority, text.into()));
}

fn body_start(node: &Node, text: &str) -> usize {
    for child in node.children.iter().skip(1) {
        if !child.list && child.atom(text).starts_with('$') {
            continue;
        }
        if child.list
            && matches!(
                child.head(text),
                Some("type" | "param" | "result" | "local" | "export")
            )
        {
            continue;
        }
        return child.start;
    }
    node.end - 1
}

#[derive(Default)]
struct Types {
    arrays: HashMap<String, u64>,
    structures: HashMap<String, u64>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StaticCharge {
    pub work: u64,
    pub allocation: u64,
}

fn constant_i32(node: &Node, text: &str) -> u64 {
    assert_eq!(
        node.head(text),
        Some("i32.const"),
        "nonconstant allocation in global initializer"
    );
    let value = node.children[1].atom(text);
    if let Some(hex) = value.strip_prefix("0x") {
        u32::from_str_radix(hex, 16).expect("invalid global array size") as u64
    } else {
        value.parse::<u32>().expect("invalid global array size") as u64
    }
}

fn charge_constants(node: &Node, text: &str, types: &Types, charge: &mut StaticCharge) {
    if !node.list {
        return;
    }
    let head = node.head(text).unwrap_or("");
    let (work, words) = match head {
        "struct.new" => {
            let fields = (node.children.len() - 2) as u64;
            (1 + fields, 4 + fields)
        }
        "struct.new_default" => {
            let fields = types.structures[node.children[1].atom(text)];
            (1 + fields, 4 + fields)
        }
        "array.new_fixed" => {
            let count = node.children[2]
                .atom(text)
                .parse::<u64>()
                .expect("array fixed size");
            (
                1 + count,
                4 + (count * types.arrays[node.children[1].atom(text)]).div_ceil(8),
            )
        }
        "array.new" | "array.new_default" | "array.new_data" | "array.new_elem" => {
            let count = constant_i32(node.children.last().expect("array length"), text);
            (
                1 + count,
                4 + (count * types.arrays[node.children[1].atom(text)]).div_ceil(8),
            )
        }
        _ => (0, 0),
    };
    if words != 0 {
        charge.work = charge
            .work
            .checked_add(work)
            .expect("static macro work overflow");
        charge.allocation = charge
            .allocation
            .checked_add(words)
            .expect("static macro allocation overflow");
    }
    for child in &node.children {
        charge_constants(child, text, types, charge);
    }
}

/// Cost of GC allocations in Wasm constant expressions. The host charges it
/// before instantiation, because a Wasm initializer cannot call a hook.
pub fn static_charge(text: &str) -> StaticCharge {
    let mut parser = Parser { text, at: 0 };
    let module = parser.node();
    parser.skip();
    assert_eq!(parser.at, text.len(), "trailing generated WAT");
    assert_eq!(module.head(text), Some("module"));
    let mut types = Types::default();
    collect_types(&module, text, &mut types);
    let mut result = StaticCharge::default();
    for global in module
        .children
        .iter()
        .filter(|n| n.head(text) == Some("global"))
    {
        charge_constants(global, text, &types, &mut result);
    }
    for memory in module
        .children
        .iter()
        .filter(|n| n.head(text) == Some("memory"))
    {
        let pages = memory
            .children
            .iter()
            .skip(1)
            .filter(|n| !n.list)
            .find_map(|n| n.atom(text).parse::<u64>().ok())
            .expect("memory has no initial size");
        result.work = result
            .work
            .checked_add(pages.checked_mul(65536).expect("memory work overflow"))
            .expect("static macro work overflow");
        result.allocation = result
            .allocation
            .checked_add(pages.checked_mul(8192).expect("memory size overflow"))
            .expect("static macro allocation overflow");
    }
    result
}

fn find_form<'a>(node: &'a Node, text: &str, kind: &str) -> Option<&'a Node> {
    if node.head(text) == Some(kind) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_form(child, text, kind))
}

fn collect_types(node: &Node, text: &str, types: &mut Types) {
    if node.head(text) == Some("type") && node.children.len() >= 3 && !node.children[1].list {
        let name = node.children[1].atom(text);
        let body = &node.children[2];
        if let Some(array) = find_form(body, text, "array") {
            let mut value = &array.children[1];
            if value.head(text) == Some("mut") {
                value = &value.children[1];
            }
            let width = if value.list {
                8
            } else {
                match value.atom(text) {
                    "i8" => 1,
                    "i16" => 2,
                    "i32" | "f32" => 4,
                    "i64" | "f64" | "eqref" | "externref" | "funcref" => 8,
                    other => panic!("unknown WAT array storage type {other}"),
                }
            };
            types.arrays.insert(name.to_owned(), width);
        } else if let Some(structure) = find_form(body, text, "struct") {
            let count = structure
                .children
                .iter()
                .skip(1)
                .map(|field| {
                    assert_eq!(field.head(text), Some("field"), "invalid WAT field");
                    field
                        .children
                        .iter()
                        .skip(1)
                        .filter(|n| n.list || !n.atom(text).starts_with('$'))
                        .count() as u64
                })
                .sum();
            types.structures.insert(name.to_owned(), count);
        }
    }
    for child in &node.children {
        collect_types(child, text, types);
    }
}

fn walk(node: &Node, text: &str, edits: &mut Inserts, types: &Types) {
    if !node.list {
        return;
    }
    let head = node.head(text).unwrap_or("");
    match head {
        "loop" => insert(
            edits,
            body_start(node, text),
            1,
            "(call $macro_charge (i64.const 1) (i64.const 0))\n",
        ),
        "struct.new" | "struct.new_default" | "array.new_fixed" => {
            let ty = node.children[1].atom(text);
            let (work, fields) = if head == "array.new_fixed" {
                let count = node.children[2]
                    .atom(text)
                    .parse::<u64>()
                    .expect("array fixed size");
                (
                    count,
                    (count * types.arrays.get(ty).expect("missing array storage type")).div_ceil(8),
                )
            } else if head == "struct.new" {
                let n = (node.children.len() - 2) as u64;
                (n, n)
            } else {
                let n = *types.structures.get(ty).expect("missing structure type");
                (n, n)
            };
            insert(
                edits,
                node.start,
                2,
                format!(
                    "(block (result (ref {ty})) (call $macro_charge (i64.const {work}) (i64.const {})) ",
                    fields + 4
                ),
            );
            insert(edits, node.end, 0, ")");
        }
        "array.new" | "array.new_default" | "array.new_data" | "array.new_elem" => {
            let length = node.children.last().expect("array length");
            let ty = node.children[1].atom(text);
            let helper = match types.arrays.get(ty).expect("missing array storage type") {
                1 => "$macro_array_bytes",
                2 => "$macro_array_quarters",
                4 => "$macro_array_halves",
                8 => "$macro_array_words",
                width => panic!("invalid array element width {width}"),
            };
            insert(edits, length.start, 2, format!("(call {helper} "));
            insert(edits, length.end, 0, ")");
        }
        "array.copy" | "array.fill" | "memory.copy" | "memory.fill" => {
            let length = node.children.last().expect("copy length");
            insert(edits, length.start, 2, "(call $macro_copy_units ");
            insert(edits, length.end, 0, ")");
        }
        "memory.grow" => {
            let length = node.children.last().expect("memory growth");
            insert(edits, length.start, 2, "(call $macro_memory_pages ");
            insert(edits, length.end, 0, ")");
        }
        "return_call"
            if node
                .children
                .get(1)
                .is_some_and(|n| !n.list && n.atom(text) == "$shadow_enter") =>
        {
            // shadow_site can synthesize a frame when no source frame exists.
            insert(edits, node.start, 1, "(call $macro_enter (i32.const 1))\n");
        }
        _ => {}
    }
    for child in &node.children {
        walk(child, text, edits, types);
    }
}

/// Instrument the complete generated WAT before Binaryen sees the module.
/// Constant initializers are accounted separately by `static_charge`, before
/// instantiation, because Wasm constant expressions cannot call a hook.
pub fn instrument(text: &str) -> String {
    let mut p = Parser { text, at: 0 };
    let module = p.node();
    p.skip();
    assert_eq!(p.at, text.len(), "trailing generated WAT");
    assert_eq!(module.head(text), Some("module"));
    let mut edits = Inserts::new();
    let mut types = Types::default();
    collect_types(&module, text, &mut types);
    // Imports must precede definitions; helpers can appear after all fields.
    insert(&mut edits, module.children[0].end, 1, IMPORTS);
    insert(&mut edits, module.end - 1, 1, HELPERS);
    for function in module
        .children
        .iter()
        .filter(|n| n.head(text) == Some("func"))
    {
        let name = function
            .children
            .get(1)
            .filter(|n| !n.list)
            .map(|n| n.atom(text))
            .unwrap_or("");
        let start = body_start(function, text);
        insert(
            &mut edits,
            start,
            1,
            "(call $macro_charge (i64.const 1) (i64.const 0))\n",
        );
        match name {
            "$shadow_leave" | "$shadow_leave_tail" => {
                insert(&mut edits, start, 1, "(call $macro_leave)\n")
            }
            "$shadow_restore" => insert(
                &mut edits,
                start,
                1,
                "(call $macro_restore (local.get $depth))\n",
            ),
            _ => {}
        }
        for node in &function.children {
            walk(node, text, &mut edits, &types);
        }
    }
    let mut out = String::with_capacity(text.len() + edits.len() * 32);
    let mut previous = 0;
    for (at, mut changes) in edits {
        out.push_str(&text[previous..at]);
        changes.sort_by_key(|c| c.0);
        for (_, change) in changes {
            out.push_str(&change);
        }
        previous = at;
    }
    out.push_str(&text[previous..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covers_nested_allocations_copies_and_loops_without_evaluating_lengths_twice() {
        let text = r#"(module (type $a (array (mut i32))) (type $s (struct (field (ref $a))))
          (func $f (param $n i32) (result (ref $s))
            (loop $l (array.copy $a $a (array.new_default $a (local.get $n)) (i32.const 0)
              (array.new_default $a (local.get $n)) (i32.const 0) (local.get $n)))
            (struct.new $s (array.new_fixed $a 2 (i32.const 1) (i32.const 2)))))"#;
        let output = instrument(text);
        assert_eq!(
            output
                .matches("(call $macro_array_halves (local.get $n))")
                .count(),
            2
        );
        assert!(output.contains("(call $macro_copy_units (local.get $n))"));
        assert!(output.contains("(loop $l (call $macro_charge"));
        wat::parse_str(output).unwrap();
    }

    #[test]
    fn static_and_default_allocations_use_declared_storage_widths() {
        let text = r#"(module (type $a (array (mut i8)))
          (type $s (struct (field i32) (field i64)))
          (global $g (ref $a) (array.new_default $a (i32.const 9)))
          (func $f (result (ref $s)) (struct.new_default $s)))"#;
        assert_eq!(
            static_charge(text),
            StaticCharge {
                work: 10,
                allocation: 6
            }
        );
        let output = instrument(text);
        assert!(output.contains("(i64.const 2) (i64.const 6)) (struct.new_default $s)"));
        wat::parse_str(output).unwrap();
        assert_eq!(
            static_charge("(module (memory (export \"memory\") 2))"),
            StaticCharge {
                work: 131072,
                allocation: 16384
            }
        );
    }
}
