mod common;
use common::{close, j, number_jokers, snap_json, table, to_snapshot};
use mortemqol_engine::score::score_play;
use mortemqol_engine::snapshot::Snapshot;
use serde_json::{json, Value};

fn plays() -> Vec<Vec<usize>> {
    vec![vec![0], vec![0, 1], vec![1, 2, 3], vec![0, 1, 2, 3, 4], vec![3, 4, 5, 6, 7]]
}

fn identity(s: &Snapshot) -> Vec<usize> {
    (0..s.jokers.len()).collect()
}

#[test]
fn floor_le_expected_le_ceiling() {
    for s in table() {
        for p in plays() {
            let r = score_play(&s, &p, &identity(&s));
            assert!(r.floor <= r.expected + 1e-6, "{} > {}", r.floor, r.expected);
            assert!(r.expected <= r.ceiling + 1e-6, "{} > {}", r.expected, r.ceiling);
        }
    }
}

#[test]
fn deterministic_setups_have_equal_range() {
    for s in table() {
        for p in plays() {
            let r = score_play(&s, &p, &identity(&s));
            if !r.has_chance {
                close(r.floor, r.ceiling);
                close(r.floor, r.expected);
            }
        }
    }
}

#[test]
fn score_play_is_repeatable() {
    for s in table() {
        for p in plays() {
            let (a, b) = (score_play(&s, &p, &identity(&s)), score_play(&s, &p, &identity(&s)));
            assert_eq!(a.expected.to_bits(), b.expected.to_bits());
        }
    }
}

#[test]
fn additive_only_joker_orders_score_equally() {
    let mut v = snap_json("AS AH");
    v["jokers"] = Value::Array(number_jokers(vec![
        j("j_joker", json!({"mult": 4})),
        j("j_red_card", json!({"mult": 6})),
        j("j_stuntman", json!({"extra": {"chip_mod": 250}})),
    ]));
    let s = to_snapshot(&v);
    let a = score_play(&s, &[0, 1], &[0, 1, 2]).expected;
    let b = score_play(&s, &[0, 1], &[2, 1, 0]).expected;
    close(a, b);
}
