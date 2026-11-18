use ilang_shard::{Unit, plan, plan_cases, plan_json, with_median_missing};
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

/// 200 units with uneven costs (0.1 s to about 40 s) and one of 400 s.
fn units() -> Vec<Unit> {
    let mut units: Vec<Unit> = (0..200u64)
        .map(|i| Unit {
            key: format!("unit-{i:03}"),
            cost: 0.1 + ((i * 7919) % 401) as f64 / 10.0,
        })
        .collect();
    units.push(Unit {
        key: "huge".into(),
        cost: 400.0,
    });
    units
}

const BUDGET: f64 = 270.0;

#[test]
fn the_union_is_every_unit_exactly_once() {
    let units = units();
    let planned = plan(&units, BUDGET);
    let mut all: Vec<&str> = planned
        .shards
        .iter()
        .flat_map(|s| s.keys.iter().map(String::as_str))
        .collect();
    all.sort();
    let mut expected: Vec<&str> = units.iter().map(|u| u.key.as_str()).collect();
    expected.sort();
    assert_eq!(all, expected);
}

#[test]
fn shards_do_not_overlap() {
    let planned = plan(&units(), BUDGET);
    let mut seen = BTreeSet::new();
    for shard in &planned.shards {
        assert!(!shard.keys.is_empty(), "shard {} is empty", shard.index);
        for key in &shard.keys {
            assert!(seen.insert(key.clone()), "{key} is in two shards");
        }
    }
    let indexes: Vec<usize> = planned.shards.iter().map(|s| s.index).collect();
    assert_eq!(indexes, (0..planned.shards.len()).collect::<Vec<_>>());
}

#[test]
fn every_shard_is_within_budget_unless_the_unit_is_reported() {
    let units = units();
    let planned = plan(&units, BUDGET);
    let over: Vec<&str> = planned.over_budget.iter().map(|u| u.key.as_str()).collect();
    assert_eq!(
        over,
        ["huge"],
        "the over-budget units are exactly those costing more than the budget"
    );
    for shard in &planned.shards {
        let sum: f64 = shard
            .keys
            .iter()
            .map(|k| units.iter().find(|u| &u.key == k).unwrap().cost)
            .sum();
        assert!(
            (sum - shard.seconds).abs() < 1e-6,
            "shard {} reports {} for {sum}",
            shard.index,
            shard.seconds
        );
        if shard.seconds > BUDGET {
            assert_eq!(
                shard.keys,
                ["huge"],
                "shard {} is over budget with {:?}",
                shard.index,
                shard.keys
            );
        }
    }
    assert!(
        planned
            .shards
            .iter()
            .any(|s| s.keys.iter().any(|k| k == "huge")),
        "an over-budget unit is still assigned"
    );
}

#[test]
fn the_shard_count_is_ceiling_total_over_budget() {
    let units: Vec<Unit> = ["a", "b", "c"]
        .iter()
        .map(|k| Unit {
            key: k.to_string(),
            cost: 0.6,
        })
        .collect();
    assert_eq!(plan(&units, 1.0).shards.len(), 2);
    assert_eq!(plan(&units, 0.5).shards.len(), 3);
    assert_eq!(plan(&units, 10.0).shards.len(), 1);
}

#[test]
fn every_shard_fits_the_budget_when_no_unit_is_over() {
    let units: Vec<Unit> = (0..100)
        .map(|i| Unit {
            key: format!("u{i:03}"),
            cost: 1.0,
        })
        .collect();
    let planned = plan(&units, 10.0);
    assert_eq!(planned.shards.len(), 10);
    for shard in &planned.shards {
        assert!(
            shard.seconds <= 10.0,
            "shard {} has {} s",
            shard.index,
            shard.seconds
        );
    }
    assert!(planned.over_budget.is_empty());
}

#[test]
fn over_budget_units_are_reported_with_the_plan() {
    let mut units = units();
    units.push(Unit {
        key: "over".into(),
        cost: 300.0,
    });
    let planned = plan(&units, 270.0);
    let reported: Vec<&str> = planned.over_budget.iter().map(|u| u.key.as_str()).collect();
    assert!(reported.contains(&"over"), "{reported:?}");
    assert!(
        planned
            .shards
            .iter()
            .any(|s| s.keys.iter().any(|k| k == "over"))
    );
    let json = plan_json(&planned, 270.0);
    assert!(json.contains("{\"key\":\"over\",\"cost\":300}"), "{json}");
}

#[test]
fn the_same_input_gives_the_same_plan() {
    let first = plan_json(&plan(&units(), BUDGET), BUDGET);
    let mut reversed = units();
    reversed.reverse();
    let second = plan_json(&plan(&reversed, BUDGET), BUDGET);
    assert_eq!(first, second);
    assert_eq!(first, plan_json(&plan(&units(), BUDGET), BUDGET));
}

#[test]
fn ties_go_by_key_and_then_to_the_lowest_shard() {
    let units: Vec<Unit> = ["d", "c", "b", "a"]
        .iter()
        .map(|k| Unit {
            key: k.to_string(),
            cost: 1.0,
        })
        .collect();
    let planned = plan(&units, 2.0);
    let keys: Vec<Vec<String>> = planned.shards.iter().map(|s| s.keys.clone()).collect();
    assert_eq!(
        keys,
        [
            vec!["a".to_string(), "c".into()],
            vec!["b".into(), "d".into()]
        ]
    );
}

#[test]
fn a_missing_cost_uses_the_set_median() {
    let units = with_median_missing(&[
        ("a".into(), Some(1.0)),
        ("b".into(), None),
        ("c".into(), Some(9.0)),
        ("d".into(), Some(4.0)),
    ]);
    assert_eq!(
        units[1],
        Unit {
            key: "b".into(),
            cost: 4.0
        }
    );
    let even = with_median_missing(&[
        ("a".into(), Some(1.0)),
        ("b".into(), Some(3.0)),
        ("c".into(), None),
    ]);
    assert_eq!(even[2].cost, 2.0);
    let none = with_median_missing(&[("a".into(), None), ("b".into(), None)]);
    assert!(none.iter().all(|u| u.cost == 1.0));
}

#[test]
fn shard_count_follows_the_budget_not_a_constant() {
    let units: Vec<Unit> = units().into_iter().filter(|u| u.key != "huge").collect();
    let wide = plan(&units, 200.0).shards.len();
    let narrow = plan(&units, 100.0).shards.len();
    assert!(wide >= 10, "{wide}");
    // Each count is ceil(total / budget), raised a little where the costs
    // do not pack evenly, so the ratio is close to 2 but not exactly 2.
    let ratio = narrow as f64 / wide as f64;
    assert!(
        (1.8..=2.2).contains(&ratio),
        "half the budget gives {narrow} shards against {wide}"
    );
}

fn table(name: &str) -> String {
    let path = format!("{}/{name}.json", env!("CARGO_TARGET_TMPDIR"));
    std::fs::write(
        &path,
        r#"{"version":1,"budget_seconds":3,"sets":{"s":{"keys":["a","b","c","d"],"costs":{"a":2,"b":2,"c":1}}}}"#,
    )
    .unwrap();
    path
}

fn shard(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_i-shard"))
        .args(args)
        .output()
        .unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn the_cli_prints_the_plan_keys_and_matrix() {
    let costs = table("cli");
    // d takes the median 2: total 7 s, 3 shards of at most 3 s.
    let (code, out, _) = shard(&["plan", "--costs", &costs, "--set", "s"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        "{\"budget_seconds\":3,\"shards\":[{\"index\":0,\"seconds\":3,\"keys\":[\"a\",\"c\"]},\
         {\"index\":1,\"seconds\":2,\"keys\":[\"b\"]},{\"index\":2,\"seconds\":2,\"keys\":[\"d\"]}],\"over_budget\":[],\"missing_costs\":[\"d\"]}\n"
    );
    let (code, out, _) = shard(&[
        "keys", "--costs", &costs, "--set", "s", "--shard", "0", "--budget", "3",
    ]);
    assert_eq!((code, out.as_str()), (0, "a\nc\n"));
    let (code, out, _) = shard(&[
        "matrix",
        "--costs",
        &costs,
        "--set",
        "s",
        "--budget",
        "7",
        "--name-prefix",
        "test (s)",
    ]);
    assert_eq!(
        (code, out.as_str()),
        (0, "[{\"index\":0,\"of\":1,\"name\":\"test (s), 1/1\"}]\n")
    );
}

#[test]
fn an_out_of_range_shard_is_rejected() {
    let costs = table("range");
    let (code, out, err) = shard(&[
        "keys", "--costs", &costs, "--set", "s", "--shard", "3", "--budget", "3",
    ]);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert!(
        err.contains("shard 3 is out of range") && err.contains("3 shard(s)"),
        "{err}"
    );
}

#[test]
fn bad_input_is_rejected_with_a_message() {
    let costs = table("bad");
    let (code, _, err) = shard(&["plan", "--costs", &costs, "--set", "nope"]);
    assert!(code == 1 && err.contains("unknown set \"nope\""), "{err}");
    let (code, _, err) = shard(&["plan", "--costs", &costs, "--set", "s", "--budget", "0"]);
    assert!(code == 1 && err.contains("greater than 0"), "{err}");
    let (code, _, err) = shard(&["plan", "--costs", "/nonexistent/costs.json", "--set", "s"]);
    assert!(
        code == 1 && err.contains("cannot read the cost table"),
        "{err}"
    );
    let (code, _, err) = shard(&["keys", "--costs", &costs, "--set", "s"]);
    assert!(code == 2 && err.contains("--shard is required"), "{err}");
}

fn strings(keys: &[&str]) -> Vec<String> {
    keys.iter().map(|k| k.to_string()).collect()
}

#[test]
fn a_runner_plans_its_own_cases_with_the_table_costs() {
    let costs = table("cases");
    // `e` is not in the table and takes the median of 2, 2 and 1; 7 s at a
    // budget of 3 s is 3 shards, whatever the fallback count.
    let keys = strings(&["c", "e", "a", "b"]);
    let planned = plan_cases(Path::new(&costs), "s", &keys, 7);
    assert_eq!(planned.fallback, None);
    assert_eq!(planned.budget_seconds, 3.0);
    let shards: Vec<Vec<String>> = planned.plan.shards.iter().map(|s| s.keys.clone()).collect();
    assert_eq!(
        shards,
        vec![strings(&["a", "c"]), strings(&["b"]), strings(&["e"])]
    );
    assert_eq!(planned.plan.missing_costs, strings(&["e"]));
    assert_eq!(planned.unlisted, strings(&["e"]));
    assert_eq!(planned.absent, strings(&["d"]));
    assert!(planned.partition_errors(&keys).is_empty());
    assert_eq!(planned.shard_keys(1, 3, "test"), strings(&["b"]).as_slice());
    assert_eq!(
        planned.notes("test").len(),
        2,
        "{:?}",
        planned.notes("test")
    );
}

#[test]
#[should_panic(expected = "but 4 were asked for")]
fn a_runner_asked_for_another_count_fails() {
    let costs = table("count");
    let planned = plan_cases(Path::new(&costs), "s", &strings(&["a", "b", "c", "d"]), 4);
    planned.shard_keys(0, 4, "test");
}

#[test]
fn without_the_table_the_cases_are_equal_and_the_count_is_asked() {
    let costs = format!("{}/no-such-table.json", env!("CARGO_TARGET_TMPDIR"));
    let keys = strings(&["e", "d", "c", "b", "a"]);
    let planned = plan_cases(Path::new(&costs), "s", &keys, 2);
    assert!(planned.fallback.is_some());
    assert_eq!(
        planned.notes("test").len(),
        1,
        "{:?}",
        planned.notes("test")
    );
    assert_eq!(planned.plan.shards.len(), 2);
    assert!(planned.partition_errors(&keys).is_empty());
    assert_eq!(
        planned.shard_keys(1, 2, "test"),
        strings(&["b", "d"]).as_slice()
    );
    // Never more shards than cases.
    assert_eq!(
        plan_cases(Path::new(&costs), "s", &keys, 9)
            .plan
            .shards
            .len(),
        5
    );
}

#[test]
fn partition_errors_name_the_keys() {
    let costs = table("partition");
    let keys = strings(&["a", "b", "c", "d"]);
    let mut planned = plan_cases(Path::new(&costs), "s", &keys, 1);
    planned.plan.shards[1].keys.push("a".into());
    let errors = planned.partition_errors(&strings(&["a", "b", "c", "x"]));
    assert_eq!(
        errors,
        vec![
            "a is in shard 0 and in shard 1".to_string(),
            "d is in a shard but is not a case".to_string(),
            "x is in no shard".to_string(),
        ]
    );
}
