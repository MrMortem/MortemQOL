#![allow(dead_code)]
//! Fixture builders. Cards are written like "AS" (rank then suit) or "10H".
use mortemqol_engine::snapshot::{self, Card, Snapshot};
use serde_json::{json, Value};

pub fn card_json(code: &str, id: i64) -> Value {
    let (rank, suit) = code.split_at(code.len() - 1);
    let rank = match rank {
        "A" => 14,
        "K" => 13,
        "Q" => 12,
        "J" => 11,
        n => n.parse::<i64>().unwrap(),
    };
    let suit = match suit {
        "S" => "Spades",
        "H" => "Hearts",
        "C" => "Clubs",
        "D" => "Diamonds",
        _ => panic!("bad suit"),
    };
    json!({"id": id, "rank": rank, "suit": suit})
}

pub fn card(code: &str) -> Card {
    serde_json::from_value(card_json(code, 1)).unwrap()
}

pub fn cards(codes: &str) -> Vec<Card> {
    codes
        .split_whitespace()
        .enumerate()
        .map(|(i, c)| serde_json::from_value(card_json(c, i as i64 + 1)).unwrap())
        .collect()
}

/// Hand levels at level 1, with the game's base and per-level values.
pub fn levels_json() -> Value {
    let rows = [
        ("Flush Five", 160, 16, 50, 3),
        ("Flush House", 140, 14, 40, 4),
        ("Five of a Kind", 120, 12, 35, 3),
        ("Straight Flush", 100, 8, 40, 4),
        ("Four of a Kind", 60, 7, 30, 3),
        ("Full House", 40, 4, 25, 2),
        ("Flush", 35, 4, 15, 2),
        ("Straight", 30, 4, 30, 3),
        ("Three of a Kind", 30, 3, 20, 2),
        ("Two Pair", 20, 2, 20, 1),
        ("Pair", 10, 2, 15, 1),
        ("High Card", 5, 1, 10, 1),
    ];
    let mut map = serde_json::Map::new();
    for (name, chips, mult, l_chips, l_mult) in rows {
        map.insert(
            name.to_string(),
            json!({"level":1,"chips":chips,"mult":mult,"l_chips":l_chips,"l_mult":l_mult,
                   "played":0,"played_round":0,"visible":true}),
        );
    }
    Value::Object(map)
}

/// A snapshot JSON value with the given hand and no jokers on a Small Blind.
pub fn snap_json(hand: &str) -> Value {
    let hand: Vec<Value> =
        hand.split_whitespace().enumerate().map(|(i, c)| card_json(c, i as i64 + 1)).collect();
    json!({
        "schema": 1, "hand": hand, "jokers": [], "levels": levels_json(),
        "blind": {"key": "bl_small", "disabled": false, "target": 300},
        "round": {"hands_left": 4, "discards_left": 3, "dollars": 4, "chips": 0},
        "deck": {"back": "b_red", "remaining": 44, "total": 52},
        "vouchers": [], "consumables": [], "lobby": false
    })
}

pub fn to_snapshot(v: &Value) -> Snapshot {
    snapshot::parse(v.to_string().as_bytes()).unwrap()
}

pub fn snap(hand: &str) -> Snapshot {
    to_snapshot(&snap_json(hand))
}

/// Scores `play` (indexes into the hand) with the snapshot's joker order.
pub fn score(s: &Snapshot, play: &[usize]) -> mortemqol_engine::score::Scored {
    let order: Vec<usize> = (0..s.jokers.len()).collect();
    mortemqol_engine::score::score_play(s, play, &order)
}

pub fn j(key: &str, ab: Value) -> Value {
    json!({"id": 100, "key": key, "ab": ab, "sell": 2, "rarity": 1})
}

/// Scores every card in `hand` except the last `held`, with these jokers.
pub fn play(hand: &str, held: usize, jokers: Vec<Value>) -> mortemqol_engine::score::Scored {
    let mut v = snap_json(hand);
    v["jokers"] = Value::Array(number_jokers(jokers));
    let s = to_snapshot(&v);
    let idx: Vec<usize> = (0..s.hand.len() - held).collect();
    score(&s, &idx)
}

/// Gives each joker a distinct id.
pub fn number_jokers(jokers: Vec<Value>) -> Vec<Value> {
    jokers
        .into_iter()
        .enumerate()
        .map(|(i, mut v)| {
            v["id"] = json!(100 + i);
            v
        })
        .collect()
}

pub fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}

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
