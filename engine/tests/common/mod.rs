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
