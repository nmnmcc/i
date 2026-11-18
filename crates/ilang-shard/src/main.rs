//! `i-shard`: plans the CI shards of one set of the cost table.
//!
//! ```text
//! i-shard plan   --costs <path> --set <name> [--budget <seconds>]
//! i-shard keys   --costs <path> --set <name> [--budget <seconds>] --shard <K>
//! i-shard matrix --costs <path> --set <name> [--budget <seconds>] --name-prefix <text>
//! ```
//!
//! Without `--budget`, the table's `budget_seconds` is the budget. Errors go
//! to standard error with exit status 1, and wrong usage exits with 2.

use ilang_shard::{CostTable, matrix_json, plan, plan_json};
use std::process::ExitCode;

const USAGE: &str = "usage:
  i-shard plan   --costs <path> --set <name> [--budget <seconds>]
  i-shard keys   --costs <path> --set <name> [--budget <seconds>] --shard <K>
  i-shard matrix --costs <path> --set <name> [--budget <seconds>] --name-prefix <text>
--budget defaults to the cost table's budget_seconds; K is 0-based.";

enum Failure {
    Usage(String),
    Error(String),
}

struct Options {
    command: String,
    costs: String,
    set: String,
    budget: Option<String>,
    shard: Option<String>,
    name_prefix: Option<String>,
}

fn options(args: &[String]) -> Result<Options, Failure> {
    let usage = |m: String| Failure::Usage(m);
    let command = args.first().ok_or_else(|| usage("no command".into()))?.clone();
    if !matches!(command.as_str(), "plan" | "keys" | "matrix") {
        return Err(usage(format!("unknown command {command:?}")));
    }
    let (mut costs, mut set, mut budget, mut shard, mut name_prefix) = (None, None, None, None, None);
    let mut rest = args[1..].iter();
    while let Some(flag) = rest.next() {
        let slot = match flag.as_str() {
            "--costs" => &mut costs,
            "--set" => &mut set,
            "--budget" => &mut budget,
            "--shard" if command == "keys" => &mut shard,
            "--name-prefix" if command == "matrix" => &mut name_prefix,
            _ => return Err(usage(format!("{command}: unknown option {flag:?}"))),
        };
        let value = rest.next().ok_or_else(|| usage(format!("{flag} needs a value")))?;
        if slot.replace(value.clone()).is_some() {
            return Err(usage(format!("{flag} is given twice")));
        }
    }
    let costs = costs.ok_or_else(|| usage("--costs is required".into()))?;
    let set = set.ok_or_else(|| usage("--set is required".into()))?;
    if command == "keys" && shard.is_none() {
        return Err(usage("keys: --shard is required".into()));
    }
    if command == "matrix" && name_prefix.is_none() {
        return Err(usage("matrix: --name-prefix is required".into()));
    }
    Ok(Options { command, costs, set, budget, shard, name_prefix })
}

fn run(args: &[String]) -> Result<String, Failure> {
    let o = options(args)?;
    let error = Failure::Error;
    let text = std::fs::read_to_string(&o.costs)
        .map_err(|e| error(format!("cannot read the cost table {}: {e}", o.costs)))?;
    let table = CostTable::parse(&text).map_err(|e| error(format!("{}: {e}", o.costs)))?;
    let set = table.sets.get(&o.set).ok_or_else(|| {
        let known: Vec<&str> = table.sets.keys().map(String::as_str).collect();
        error(format!("{}: unknown set {:?}; the sets are: {}", o.costs, o.set, known.join(", ")))
    })?;
    let budget = match &o.budget {
        Some(text) => text
            .parse::<f64>()
            .map_err(|_| error(format!("--budget must be a number of seconds, not {text:?}")))?,
        None => table
            .budget_seconds
            .ok_or_else(|| error(format!("{} has no budget_seconds; give --budget", o.costs)))?,
    };
    if !(budget.is_finite() && budget > 0.0) {
        return Err(error(format!("the budget must be greater than 0 seconds, not {budget}")));
    }
    let mut planned = plan(&set.units(), budget);
    planned.missing_costs = set.missing();
    if !planned.missing_costs.is_empty() {
        eprintln!(
            "i-shard: {} unit(s) have no measured cost and use the set median: {}",
            planned.missing_costs.len(),
            planned.missing_costs.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
        );
    }
    if table.measured_at.is_none() {
        eprintln!("i-shard: {} has no measured_at; the costs may be stale, which affects balance only", o.costs);
    }
    Ok(match o.command.as_str() {
        "plan" => plan_json(&planned, budget) + "\n",
        "keys" => {
            let text = o.shard.unwrap();
            let count = planned.shards.len();
            let k = text.parse::<usize>().map_err(|_| {
                error(format!("--shard must be a shard index from 0 to {}, not {text:?}", count - 1))
            })?;
            let shard = planned.shards.get(k).ok_or_else(|| {
                error(format!(
                    "shard {k} is out of range: set {:?} has {count} shard(s) at a budget of {budget} s, so K is 0 to {}",
                    o.set,
                    count - 1
                ))
            })?;
            shard.keys.iter().map(|k| format!("{k}\n")).collect()
        }
        _ => matrix_json(&planned, o.name_prefix.as_deref().unwrap()) + "\n",
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        Err(Failure::Usage(message)) => {
            eprintln!("i-shard: {message}\n{USAGE}");
            ExitCode::from(2)
        }
        Err(Failure::Error(message)) => {
            eprintln!("i-shard: {message}");
            ExitCode::from(1)
        }
    }
}
