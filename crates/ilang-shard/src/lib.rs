//! Assigns units of CI work (test binaries, spec cases, projects) to shards
//! from their measured costs, so that no shard list is written by hand.
//!
//! The number of shards is `max(1, ceil(total / budget))`, raised one at a
//! time only while some shard would exceed the budget without holding a unit
//! that is over the budget on its own; it never exceeds the number of units.
//! Units are placed by longest processing time first: by cost descending,
//! ties by key ascending, each to the shard with the least load, ties to the
//! lowest index. The result is the same for the same input on every run.
//!
//! The cost table (`ci/costs.json`) is described in [`CostTable`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// One unit of work and its cost in seconds (greater than zero).
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub key: String,
    pub cost: f64,
}

/// One shard: its 0-based index, the sum of its costs, and its keys in
/// ascending order.
#[derive(Debug, Clone, PartialEq)]
pub struct Shard {
    pub index: usize,
    pub seconds: f64,
    pub keys: Vec<String>,
}

/// The shards, and the units whose cost alone is over the budget, by key.
/// Those units are still assigned, each to a shard of its own.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub shards: Vec<Shard>,
    pub over_budget: Vec<Unit>,
}

/// Plans the shards of `units` for `budget_seconds` per shard.
///
/// # Panics
///
/// When the budget is not a positive finite number, when a cost is not a
/// positive finite number, or when two units have the same key.
pub fn plan(units: &[Unit], budget_seconds: f64) -> Plan {
    assert!(
        budget_seconds.is_finite() && budget_seconds > 0.0,
        "the budget must be a positive number of seconds, not {budget_seconds}"
    );
    let mut seen = BTreeSet::new();
    for unit in units {
        assert!(
            unit.cost.is_finite() && unit.cost > 0.0,
            "the cost of {:?} must be a positive number of seconds, not {}",
            unit.key,
            unit.cost
        );
        assert!(seen.insert(unit.key.as_str()), "the key {:?} is given twice", unit.key);
    }

    let mut order: Vec<&Unit> = units.iter().collect();
    order.sort_by(|a, b| b.cost.total_cmp(&a.cost).then_with(|| a.key.cmp(&b.key)));
    let total: f64 = order.iter().map(|u| u.cost).sum();

    let mut over_budget: Vec<Unit> = units.iter().filter(|u| u.cost > budget_seconds).cloned().collect();
    over_budget.sort_by(|a, b| a.key.cmp(&b.key));

    let most = units.len().max(1);
    let mut count = ((total / budget_seconds).ceil() as usize).clamp(1, most);
    loop {
        let shards = assign(&order, count);
        let fits = shards
            .iter()
            .all(|s| s.seconds <= budget_seconds || s.keys.len() == 1);
        if fits || count == most {
            return Plan { shards, over_budget };
        }
        count += 1;
    }
}

fn assign(order: &[&Unit], count: usize) -> Vec<Shard> {
    let mut shards: Vec<Shard> = (0..count)
        .map(|index| Shard { index, seconds: 0.0, keys: Vec::new() })
        .collect();
    for unit in order {
        let mut least = 0;
        for (i, shard) in shards.iter().enumerate().skip(1) {
            if shard.seconds < shards[least].seconds {
                least = i;
            }
        }
        shards[least].seconds += unit.cost;
        shards[least].keys.push(unit.key.clone());
    }
    for shard in &mut shards {
        shard.keys.sort();
    }
    shards
}

/// Gives each unit without a cost the median of the costs that are present
/// (the mean of the two middle ones for an even count), or 1.0 when none is.
pub fn with_median_missing(units: &[(String, Option<f64>)]) -> Vec<Unit> {
    let mut present: Vec<f64> = units.iter().filter_map(|(_, c)| *c).collect();
    present.sort_by(f64::total_cmp);
    let median = match present.len() {
        0 => 1.0,
        n if n % 2 == 1 => present[n / 2],
        n => (present[n / 2 - 1] + present[n / 2]) / 2.0,
    };
    units
        .iter()
        .map(|(key, cost)| Unit { key: key.clone(), cost: cost.unwrap_or(median) })
        .collect()
}

/// A number of seconds as JSON, rounded to milliseconds.
pub fn seconds_json(seconds: f64) -> String {
    let rounded = (seconds * 1000.0).round() / 1000.0;
    // `-0` would be odd in a cost; a cost is positive anyway.
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    format!("{rounded}")
}

fn string_json(text: &str) -> String {
    serde_json::to_string(text).expect("a string is always JSON")
}

/// The plan as one line of JSON:
/// `{"budget_seconds":N,"shards":[{"index":0,"seconds":S,"keys":[...]}],"over_budget":[{"key":K,"cost":C}]}`.
pub fn plan_json(plan: &Plan, budget_seconds: f64) -> String {
    let mut out = format!("{{\"budget_seconds\":{},\"shards\":[", seconds_json(budget_seconds));
    for (i, shard) in plan.shards.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let keys: Vec<String> = shard.keys.iter().map(|k| string_json(k)).collect();
        let _ = write!(
            out,
            "{{\"index\":{},\"seconds\":{},\"keys\":[{}]}}",
            shard.index,
            seconds_json(shard.seconds),
            keys.join(",")
        );
    }
    out.push_str("],\"over_budget\":[");
    for (i, unit) in plan.over_budget.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let _ = write!(out, "{{\"key\":{},\"cost\":{}}}", string_json(&unit.key), seconds_json(unit.cost));
    }
    out.push_str("]}");
    out
}

/// The shards as a GitHub Actions matrix:
/// `[{"index":0,"of":N,"name":"<prefix>, 1/N"}, ...]`.
pub fn matrix_json(plan: &Plan, name_prefix: &str) -> String {
    let of = plan.shards.len();
    let items: Vec<String> = plan
        .shards
        .iter()
        .map(|s| {
            format!(
                "{{\"index\":{},\"of\":{of},\"name\":{}}}",
                s.index,
                string_json(&format!("{name_prefix}, {}/{of}", s.index + 1))
            )
        })
        .collect();
    format!("[{}]", items.join(","))
}

/// One set of the cost table: its unit keys and the measured costs of some.
#[derive(Debug, Clone, PartialEq)]
pub struct CostSet {
    pub keys: Vec<String>,
    pub costs: BTreeMap<String, f64>,
}

impl CostSet {
    /// The units of the set; a key without a cost takes the set's median.
    pub fn units(&self) -> Vec<Unit> {
        let pairs: Vec<(String, Option<f64>)> = self
            .keys
            .iter()
            .map(|k| (k.clone(), self.costs.get(k).copied()))
            .collect();
        with_median_missing(&pairs)
    }
}

/// The cost table:
///
/// ```json
/// {"version":1,"budget_seconds":270,
///  "sets":{"spec-cases":{"keys":["10.1/x.i","10.2/y"],"costs":{"10.1/x.i":0.4}}}}
/// ```
///
/// `keys` lists every unit of the set once; `costs` may leave keys out, and
/// those take the median of the set. A cost for a key that is not listed is
/// ignored.
#[derive(Debug, Clone, PartialEq)]
pub struct CostTable {
    pub budget_seconds: Option<f64>,
    pub sets: BTreeMap<String, CostSet>,
}

impl CostTable {
    pub fn parse(text: &str) -> Result<CostTable, String> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
        let root = value.as_object().ok_or("the top level is not an object")?;
        match root.get("version").and_then(|v| v.as_u64()) {
            Some(1) => {}
            other => return Err(format!("version must be 1, not {other:?}")),
        }
        let budget_seconds = match root.get("budget_seconds") {
            None => None,
            Some(v) => Some(v.as_f64().ok_or("budget_seconds is not a number")?),
        };
        let raw_sets = root
            .get("sets")
            .and_then(|s| s.as_object())
            .ok_or("sets is missing or not an object")?;
        let mut sets = BTreeMap::new();
        for (name, raw) in raw_sets {
            let set = raw.as_object().ok_or_else(|| format!("set {name:?} is not an object"))?;
            let raw_keys = set
                .get("keys")
                .and_then(|k| k.as_array())
                .ok_or_else(|| format!("set {name:?} has no keys list"))?;
            let mut keys = Vec::with_capacity(raw_keys.len());
            let mut seen = BTreeSet::new();
            for key in raw_keys {
                let key = key
                    .as_str()
                    .ok_or_else(|| format!("set {name:?} has a key that is not a string"))?;
                if !seen.insert(key) {
                    return Err(format!("set {name:?} lists the key {key:?} twice"));
                }
                keys.push(key.to_string());
            }
            let mut costs = BTreeMap::new();
            if let Some(raw_costs) = set.get("costs") {
                let raw_costs = raw_costs
                    .as_object()
                    .ok_or_else(|| format!("the costs of set {name:?} are not an object"))?;
                for (key, cost) in raw_costs {
                    let cost = cost
                        .as_f64()
                        .filter(|c| c.is_finite() && *c > 0.0)
                        .ok_or_else(|| format!("set {name:?}: the cost of {key:?} is not a positive number"))?;
                    costs.insert(key.clone(), cost);
                }
            }
            sets.insert(name.clone(), CostSet { keys, costs });
        }
        Ok(CostTable { budget_seconds, sets })
    }
}
