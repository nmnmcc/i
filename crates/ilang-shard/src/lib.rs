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
use std::path::Path;

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
    /// Units that had no measured cost and took the set median.
    pub missing_costs: Vec<String>,
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
    let count = ((total / budget_seconds).ceil() as usize).clamp(1, most);
    let shards = assign(&order, count);
    Plan {
        shards,
        over_budget,
        missing_costs: Vec::new(),
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

/// The plan of the cases that a test runner found, and where its costs came
/// from.
#[derive(Debug, Clone, PartialEq)]
pub struct CasePlan {
    pub plan: Plan,
    pub budget_seconds: f64,
    /// Why every case costs 1 s, when the cost table could not be read.
    pub fallback: Option<String>,
    /// Cases that the runner found and the set does not list, sorted.
    pub unlisted: Vec<String>,
    /// Keys of the set that the runner did not find, sorted.
    pub absent: Vec<String>,
}

/// Plans the cases `keys` that a test runner found, with the costs of the set
/// `set_name` of the cost table at `costs_path` and the table's budget.
///
/// The keys are sorted first, so the plan does not depend on the order in
/// which the runner found them. A case without a measured cost takes the
/// median of the measured ones, as in [`with_median_missing`].
///
/// When the table cannot be read (a checkout without it), every case costs
/// 1 s and the plan has `fallback_shards` shards (or one per case, when there
/// are fewer cases), and `fallback` says why.
///
/// # Panics
///
/// When the table can be read but is not valid, has no set `set_name` or no
/// `budget_seconds`; when `fallback_shards` is 0; when a key is given twice.
pub fn plan_cases(costs_path: &Path, set_name: &str, keys: &[String], fallback_shards: usize) -> CasePlan {
    assert!(fallback_shards > 0, "a plan has at least one shard");
    let mut keys = keys.to_vec();
    keys.sort();
    if let Some(pair) = keys.windows(2).find(|pair| pair[0] == pair[1]) {
        panic!("the case {:?} is found twice", pair[0]);
    }
    let text = match std::fs::read_to_string(costs_path) {
        Ok(text) => text,
        Err(error) => {
            let units: Vec<Unit> = keys.iter().map(|key| Unit { key: key.clone(), cost: 1.0 }).collect();
            let order: Vec<&Unit> = units.iter().collect();
            let count = fallback_shards.min(units.len()).max(1);
            let shards = assign(&order, count);
            // The largest shard, so that no unit is over this budget.
            let budget_seconds = shards.iter().map(|s| s.seconds).fold(1.0, f64::max);
            return CasePlan {
                plan: Plan { shards, over_budget: Vec::new(), missing_costs: keys.clone() },
                budget_seconds,
                fallback: Some(format!(
                    "cannot read the cost table {}: {error}; every case costs 1 s, in {count} shard(s)",
                    costs_path.display()
                )),
                unlisted: Vec::new(),
                absent: Vec::new(),
            };
        }
    };
    let table = CostTable::parse(&text).unwrap_or_else(|e| panic!("{}: {e}", costs_path.display()));
    let set = table.sets.get(set_name).unwrap_or_else(|| {
        let known: Vec<&str> = table.sets.keys().map(String::as_str).collect();
        panic!("{}: no set {set_name:?}; the sets are: {}", costs_path.display(), known.join(", "))
    });
    let budget_seconds = table
        .budget_seconds
        .unwrap_or_else(|| panic!("{} has no budget_seconds", costs_path.display()));
    let pairs: Vec<(String, Option<f64>)> =
        keys.iter().map(|key| (key.clone(), set.costs.get(key).copied())).collect();
    let mut planned = plan(&with_median_missing(&pairs), budget_seconds);
    planned.missing_costs = pairs.iter().filter(|(_, cost)| cost.is_none()).map(|(key, _)| key.clone()).collect();
    let listed: BTreeSet<&str> = set.keys.iter().map(String::as_str).collect();
    let found: BTreeSet<&str> = keys.iter().map(String::as_str).collect();
    let unlisted = keys.iter().filter(|key| !listed.contains(key.as_str())).cloned().collect();
    let mut absent: Vec<String> = set.keys.iter().filter(|key| !found.contains(key.as_str())).cloned().collect();
    absent.sort();
    CasePlan { plan: planned, budget_seconds, fallback: None, unlisted, absent }
}

impl CasePlan {
    /// The keys of shard `index` (0-based) when the runner was asked for
    /// `count` shards.
    ///
    /// # Panics
    ///
    /// When the plan does not have exactly `count` shards (the CI matrix and
    /// the runner then read different tables or budgets), or when `index` is
    /// not below `count`. `what` names the runner in the message.
    pub fn shard_keys(&self, index: usize, count: usize, what: &str) -> &[String] {
        assert_eq!(
            self.plan.shards.len(),
            count,
            "{what}: the cost table plans {} shard(s) at a budget of {} s, but {count} were asked for; \
             the CI matrix must come from the same table (i-shard matrix)",
            self.plan.shards.len(),
            seconds_json(self.budget_seconds),
        );
        assert!(index < count, "{what}: shard {index} is out of range; the shards are 0 to {}", count - 1);
        &self.plan.shards[index].keys
    }

    /// What is wrong with the shards as a partition of `keys`: a key in two
    /// shards, a key in no shard, or a shard key that is not one of `keys`.
    /// Empty when every key is in exactly one shard.
    pub fn partition_errors(&self, keys: &[String]) -> Vec<String> {
        let mut errors = Vec::new();
        let mut shard_of: BTreeMap<&str, usize> = BTreeMap::new();
        for shard in &self.plan.shards {
            for key in &shard.keys {
                if let Some(other) = shard_of.insert(key.as_str(), shard.index) {
                    errors.push(format!("{key} is in shard {other} and in shard {}", shard.index));
                }
            }
        }
        let wanted: BTreeSet<&str> = keys.iter().map(String::as_str).collect();
        for key in shard_of.keys() {
            if !wanted.contains(key) {
                errors.push(format!("{key} is in a shard but is not a case"));
            }
        }
        for key in &wanted {
            if !shard_of.contains_key(key) {
                errors.push(format!("{key} is in no shard"));
            }
        }
        errors
    }

    /// One line for standard error about each thing that a developer should
    /// know: the fallback, cases over the budget, and cases that the runner
    /// and the table disagree on.
    pub fn notes(&self, what: &str) -> Vec<String> {
        let few = |keys: &[String]| {
            let shown: Vec<&str> = keys.iter().take(5).map(String::as_str).collect();
            let more = if keys.len() > 5 { format!(" and {} more", keys.len() - 5) } else { String::new() };
            format!("{}{more}", shown.join(", "))
        };
        let mut notes = Vec::new();
        if let Some(fallback) = &self.fallback {
            notes.push(format!("{what}: {fallback}"));
        }
        for unit in &self.plan.over_budget {
            notes.push(format!(
                "{what}: the case {} costs {} s, over the budget of {} s",
                unit.key,
                seconds_json(unit.cost),
                seconds_json(self.budget_seconds)
            ));
        }
        if !self.unlisted.is_empty() {
            notes.push(format!(
                "{what}: {} case(s) are not in the cost table and take the median: {}",
                self.unlisted.len(),
                few(&self.unlisted)
            ));
        }
        if !self.absent.is_empty() {
            notes.push(format!(
                "{what}: {} key(s) of the cost table are not cases: {}",
                self.absent.len(),
                few(&self.absent)
            ));
        }
        notes
    }
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
/// `{"budget_seconds":N,"shards":[{"index":0,"seconds":S,"keys":[...]}],"over_budget":[{"key":K,"cost":C}],"missing_costs":[K]}`.
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
    out.push_str("],\"missing_costs\":[");
    for (i, key) in plan.missing_costs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&string_json(key));
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
    /// The keys with no measured cost, sorted.
    pub fn missing(&self) -> Vec<String> {
        let mut missing: Vec<String> = self
            .keys
            .iter()
            .filter(|key| !self.costs.contains_key(*key))
            .cloned()
            .collect();
        missing.sort();
        missing
    }

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
    /// The date the costs were measured, if recorded.
    pub measured_at: Option<String>,
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
        let measured_at = match root.get("measured_at") {
            None => None,
            Some(v) => Some(v.as_str().ok_or("measured_at is not a string")?.to_string()),
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
        Ok(CostTable { budget_seconds, measured_at, sets })
    }
}
