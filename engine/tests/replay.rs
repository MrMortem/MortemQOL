//! Replays real plays captured by the mod's live check. Add a mismatch line to
//! tests/replays/*.jsonl to pin a scoring bug.
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

fn number(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::INFINITY)
}

fn check(prediction: &Value, actual: &Value, context: &str) {
    let actual = number(actual);
    let (floor, expected, ceiling) = (
        number(&prediction["floor"]),
        number(&prediction["expected"]),
        number(&prediction["ceiling"]),
    );
    if expected.is_infinite() {
        assert!(actual.is_infinite(), "{context}: predicted naneinf, game gave {actual}");
        return;
    }
    if prediction["has_chance"] == json!(true) {
        let slack = 1e-9 * ceiling.abs().max(1.0);
        assert!(
            actual >= floor - slack && actual <= ceiling + slack,
            "{context}: {actual} outside {floor}..{ceiling}"
        );
    } else {
        let slack = 1e-9 * expected.abs().max(1.0);
        assert!(
            (actual - expected).abs() <= slack,
            "{context}: predicted {expected}, game {actual}"
        );
    }
}

fn check_money(prediction: &Value, actual: f64, context: &str) {
    let (floor, ceiling) =
        (number(&prediction["money_floor"]), number(&prediction["money_ceiling"]));
    let slack = 1e-9 * ceiling.abs().max(floor.abs()).max(1.0);
    assert!(
        actual >= floor - slack && actual <= ceiling + slack,
        "{context}: money {actual} outside {floor}..{ceiling}"
    );
}

#[test]
fn all_replays_match() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/replays");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        for (i, line) in fs::read_to_string(&path).unwrap().lines().enumerate() {
            let row: Value = serde_json::from_str(line).unwrap();
            let request = json!({"snapshot": row["snapshot"], "play": row["play"]});
            let out = mortemqol_engine::score::score_request(request.to_string().as_bytes());
            let prediction: Value = serde_json::from_slice(&out.unwrap()).unwrap();
            let context = format!("{}:{}", path.display(), i + 1);
            if !row["actual"].is_null() {
                check(&prediction, &row["actual"], &context);
            }
            if let Some(money) = row["actual_money"].as_f64() {
                check_money(&prediction, money, &context);
            }
        }
    }
}

#[test]
fn check_catches_wrong_money() {
    let p = json!({"money_floor": 3.0, "money_ceiling": 3.0});
    assert!(std::panic::catch_unwind(|| check_money(&p, 4.0, "x")).is_err());
    check_money(&p, 3.0, "x");
}

#[test]
fn check_catches_a_wrong_score() {
    let p = json!({"floor": 64.0, "expected": 64.0, "ceiling": 64.0, "has_chance": false});
    assert!(std::panic::catch_unwind(|| check(&p, &json!(65.0), "x")).is_err());
    check(&p, &json!(64.0), "x");
}
