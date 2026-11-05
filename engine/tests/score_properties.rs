mod common;
use common::{close, j, number_jokers, snap_json, to_snapshot};
use mortemqol_engine::score::score_play;
use mortemqol_engine::snapshot::Snapshot;
use serde_json::{json, Value};

/// A fixed table of varied snapshots built from a small deterministic generator.
pub fn table() -> Vec<Snapshot> {
    let pool = [
        j("j_joker", json!({"mult": 4})),
        j("j_cavendish", json!({"extra": {"Xmult": 3}})),
        j("j_misprint", json!({"extra": {"min": 0, "max": 23}})),
        j("j_bloodstone", json!({"extra": {"odds": 2, "Xmult": 1.5}})),
        j("j_blueprint", json!({})),
        j("j_space", json!({"extra": 4})),
        j("j_scary_face", json!({"extra": 30})),
        j("j_baron", json!({"extra": 1.5})),
        j("j_hiker", json!({"extra": 5})),
        j("j_lucky_cat", json!({"extra": 0.25, "x_mult": 1})),
        j("j_sock_and_buskin", json!({"extra": 1})),
        j("j_bull", json!({"extra": 2})),
    ];
    let ranks = ["A", "K", "Q", "J", "10", "9", "8", "7", "6", "5", "4", "3", "2"];
    let suits = ["S", "H", "D", "C"];
    let enhancements = [None, Some("m_lucky"), Some("m_glass"), Some("m_mult"), Some("m_steel")];
    let mut seed: u64 = 12345;
    let mut next = |n: usize| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 33) as usize) % n
    };
    let mut out = Vec::new();
    for _ in 0..60 {
        let codes: Vec<String> =
            (0..8).map(|_| format!("{}{}", ranks[next(13)], suits[next(4)])).collect();
        let mut v = snap_json(&codes.join(" "));
        for i in 0..8 {
            if let Some(e) = enhancements[next(5)] {
                v["hand"][i]["enh"] = json!(e);
            }
        }
        let count = next(5);
        let jokers: Vec<Value> = (0..count).map(|_| pool[next(12)].clone()).collect();
        v["jokers"] = Value::Array(number_jokers(jokers));
        out.push(to_snapshot(&v));
    }
    out
}

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
