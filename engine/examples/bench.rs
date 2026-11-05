//! Times a full search of 8 cards and 5 order-sensitive jokers.
use mortemqol_engine::search;
use mortemqol_engine::snapshot;
use serde_json::json;
use std::time::Instant;

fn main() {
    let hand: Vec<_> = ["AS", "KS", "QH", "10H", "9H", "7D", "4C", "2C"]
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let (rank, suit) = c.split_at(c.len() - 1);
            let rank = match rank {
                "A" => 14,
                "K" => 13,
                "Q" => 12,
                n => n.parse::<i64>().unwrap(),
            };
            let suit = match suit {
                "S" => "Spades",
                "H" => "Hearts",
                "D" => "Diamonds",
                _ => "Clubs",
            };
            json!({"id": i + 1, "rank": rank, "suit": suit, "enh": if i % 3 == 0 { Some("m_glass") } else { None }})
        })
        .collect();
    let mut levels = serde_json::Map::new();
    for name in [
        "High Card",
        "Pair",
        "Two Pair",
        "Three of a Kind",
        "Straight",
        "Flush",
        "Full House",
        "Four of a Kind",
        "Straight Flush",
        "Five of a Kind",
        "Flush House",
        "Flush Five",
    ] {
        levels.insert(
            name.into(),
            json!({"level":1,"chips":30,"mult":3,"l_chips":15,"l_mult":1,"visible":true}),
        );
    }
    let jokers = json!([
        {"id": 100, "key": "j_joker", "ab": {"mult": 4}},
        {"id": 101, "key": "j_cavendish", "ab": {"extra": {"Xmult": 3}}},
        {"id": 102, "key": "j_bloodstone", "ab": {"extra": {"odds": 2, "Xmult": 1.5}}},
        {"id": 103, "key": "j_photograph", "ab": {"extra": 2}},
        {"id": 104, "key": "j_misprint", "ab": {"extra": {"min": 0, "max": 23}}}
    ]);
    let v = json!({"schema": 1, "hand": hand, "jokers": jokers, "levels": levels,
        "blind": {"key": "bl_small", "target": 300}, "deck": {"back": "b_red"},
        "round": {"hands_left": 4, "discards_left": 3, "dollars": 4}});
    let mut copier = v.clone();
    copier["jokers"][4] = json!({"id": 104, "key": "j_blueprint", "ab": {}});
    let with_copier = snapshot::parse(copier.to_string().as_bytes()).unwrap();
    let begun = Instant::now();
    search::run(&with_copier, &|| false, &mut |_| {});
    println!("with Blueprint, no order pruning: {:?}", begun.elapsed());
    let mut money = v.clone();
    money["options"] = json!({"objective": "money"});
    let money_mode = snapshot::parse(money.to_string().as_bytes()).unwrap();
    let begun = Instant::now();
    search::run(&money_mode, &|| false, &mut |_| {});
    println!("money mode: {:?}", begun.elapsed());
    let s = snapshot::parse(v.to_string().as_bytes()).unwrap();
    for _ in 0..3 {
        let begun = Instant::now();
        let mut stage_times = Vec::new();
        search::run(&s, &|| false, &mut |r| stage_times.push((r.stage, begun.elapsed())));
        println!("{:?} total {:?}", stage_times, begun.elapsed());
    }
}
