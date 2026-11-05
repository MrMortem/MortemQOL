mod common;
use common::snap_json;
use mortemqol_engine::snapshot::parse;
use serde_json::json;

#[test]
fn unknown_keys_preserved_as_strings() {
    let mut v = snap_json("AS");
    v["hand"][0]["enh"] = json!("m_modded");
    v["hand"][0]["ed"] = json!("e_modded");
    v["hand"][0]["seal"] = json!("modded");
    v["jokers"] = json!([{"id": 9, "key": "j_modded", "sell": 3}]);
    let s = parse(v.to_string().as_bytes()).unwrap();
    assert_eq!(s.hand[0].enh.as_deref(), Some("m_modded"));
    assert_eq!(s.hand[0].ed.as_deref(), Some("e_modded"));
    assert_eq!(s.hand[0].seal.as_deref(), Some("modded"));
    assert_eq!(s.jokers[0].key.as_deref(), Some("j_modded"));
}

#[test]
fn missing_optional_fields_default() {
    let mut v = snap_json("AS");
    v["jokers"] = json!([{"id": 9, "key": "j_joker"}]);
    let s = parse(v.to_string().as_bytes()).unwrap();
    assert!(!s.hand[0].debuff && !s.hand[0].forced && !s.hand[0].hidden);
    assert_eq!(s.hand[0].perma.x_mult, 1.0);
    assert_eq!(s.jokers[0].sell, 0.0);
    assert_eq!(s.jokers[0].num("mult", 7.0), 7.0);
}

#[test]
fn hidden_card_needs_no_identity() {
    let mut v = snap_json("AS");
    v["hand"] = json!([{"id": 4, "hidden": true}]);
    let s = parse(v.to_string().as_bytes()).unwrap();
    assert!(s.hand[0].hidden);
}

#[test]
fn non_numeric_ability_value_does_not_fail_parse() {
    let mut v = snap_json("AS");
    v["jokers"] = json!([{"id": 9, "key": "j_joker", "ab": {"mult": "lots", "extra": {"x": "y"}}}]);
    let s = parse(v.to_string().as_bytes()).unwrap();
    assert_eq!(s.jokers[0].num("mult", 4.0), 4.0);
    assert_eq!(s.jokers[0].extra_num("x", 2.0), 2.0);
}

#[test]
fn malformed_json_returns_err() {
    assert!(parse(b"{not json").is_err());
    assert!(parse(b"{}").is_err());
}

#[test]
fn schema_from_the_future_returns_err() {
    let mut v = snap_json("AS");
    v["schema"] = json!(2);
    assert!(parse(v.to_string().as_bytes()).is_err());
}

#[test]
fn string_rarity_from_another_mod_does_not_fail_parse() {
    let mut v = snap_json("AS");
    v["jokers"] = json!([{"id": 9, "key": "j_modded", "rarity": "cry_epic"}]);
    assert_eq!(parse(v.to_string().as_bytes()).unwrap().jokers[0].rarity, 0);
}

#[test]
fn non_numeric_fields_from_other_mods_default() {
    let mut v = snap_json("AS");
    v["jokers"] = json!([{"id": 9, "key": "j_joker", "sell": {"weird": true}}]);
    v["hand"][0]["perma"] = json!({"bonus": "x", "mult": [], "x_mult": false});
    v["levels"]["Odd"] = json!({"level": "?", "chips": {"big": 1}, "mult": "?", "played": "?"});
    let s = parse(v.to_string().as_bytes()).unwrap();
    assert_eq!(s.jokers[0].sell, 0.0);
    assert_eq!(s.hand[0].perma.bonus, 0.0);
    assert_eq!(s.hand[0].perma.x_mult, 1.0);
    assert_eq!(s.levels["Odd"].chips, 0.0);
    assert_eq!(s.levels["Odd"].level, 1.0);
}

#[test]
fn naneinf_numbers_parse_as_infinity() {
    let mut v = snap_json("AS");
    v["blind"]["target"] = json!("naneinf");
    v["round"]["dollars"] = json!("naneinf");
    v["levels"]["High Card"]["chips"] = json!("naneinf");
    let s = parse(v.to_string().as_bytes()).unwrap();
    assert!(s.blind.target.is_infinite());
    assert!(s.round.dollars.is_infinite());
    assert!(s.levels["High Card"].chips.is_infinite());
}

#[test]
fn missing_options_mean_score() {
    use mortemqol_engine::snapshot::Objective;
    let s = parse(snap_json("AS").to_string().as_bytes()).unwrap();
    assert_eq!(s.options.objective, Objective::Score);
    assert!(!s.options.discards && !s.options.seeded);
}

#[test]
fn money_objective_parses() {
    use mortemqol_engine::snapshot::Objective;
    let mut v = snap_json("AS");
    v["options"] = json!({"objective": "money", "discards": false, "seeded": false});
    assert_eq!(parse(v.to_string().as_bytes()).unwrap().options.objective, Objective::Money);
}

#[test]
fn discards_true_is_error() {
    let mut v = snap_json("AS");
    v["options"] = json!({"objective": "score", "discards": true});
    assert!(parse(v.to_string().as_bytes()).is_err());
}

#[test]
fn seeded_true_is_error() {
    let mut v = snap_json("AS");
    v["options"] = json!({"objective": "score", "seeded": true});
    assert!(parse(v.to_string().as_bytes()).is_err());
}

#[test]
fn unknown_objective_is_error() {
    let mut v = snap_json("AS");
    v["options"] = json!({"objective": "fame"});
    assert!(parse(v.to_string().as_bytes()).is_err());
}

#[test]
fn deck_contents_parse() {
    let mut v = snap_json("AS");
    v["deck"]["contents"] = json!([{"rank": 14, "suit": "Spades", "enh": "m_glass", "count": 2}]);
    let s = parse(v.to_string().as_bytes()).unwrap();
    assert_eq!(s.deck.contents[0].rank, 14);
    assert_eq!(s.deck.contents[0].count, 2.0);
    assert_eq!(s.deck.contents[0].enh.as_deref(), Some("m_glass"));
}

#[test]
fn missing_deck_contents_is_empty() {
    assert!(parse(snap_json("AS").to_string().as_bytes()).unwrap().deck.contents.is_empty());
}
