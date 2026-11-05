mod common;
use common::{score, snap, snap_json, to_snapshot};
use serde_json::json;

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

fn exact(v: &serde_json::Value, play: &[usize], want: f64) {
    let r = score(&to_snapshot(v), play);
    close(r.floor, want);
    close(r.expected, want);
    close(r.ceiling, want);
    assert!(!r.has_chance);
}

#[test]
fn pair_of_aces_level_1() {
    let r = score(&snap("AS AH KD"), &[0, 1]);
    assert_eq!(r.hand, "Pair");
    assert_eq!(r.scored, vec![1, 2]);
    assert!(r.legal);
    close(r.expected, 64.0);
}

#[test]
fn unscored_kicker_adds_nothing() {
    close(score(&snap("AS AH KD"), &[0, 1, 2]).expected, 64.0);
}

#[test]
fn high_card_lucky_range() {
    let mut v = snap_json("AS");
    v["hand"][0]["enh"] = json!("m_lucky");
    let r = score(&to_snapshot(&v), &[0]);
    close(r.floor, 16.0);
    close(r.expected, 80.0);
    close(r.ceiling, 336.0);
    assert!(r.has_chance);
}

#[test]
fn bonus_and_mult_enhancements() {
    let mut v = snap_json("AS");
    v["hand"][0]["enh"] = json!("m_bonus");
    exact(&v, &[0], 46.0);
    v["hand"][0]["enh"] = json!("m_mult");
    exact(&v, &[0], 80.0);
}

#[test]
fn stone_card_gives_50_and_no_rank_chips() {
    let mut v = snap_json("AS");
    v["hand"][0]["enh"] = json!("m_stone");
    exact(&v, &[0], 55.0);
}

#[test]
fn glass_card_x2() {
    let mut v = snap_json("AS");
    v["hand"][0]["enh"] = json!("m_glass");
    exact(&v, &[0], 32.0);
}

#[test]
fn red_seal_retriggers() {
    let mut v = snap_json("AS");
    v["hand"][0]["seal"] = json!("Red");
    exact(&v, &[0], 27.0);
}

#[test]
fn foil_adds_50_chips() {
    let mut v = snap_json("AS");
    v["hand"][0]["ed"] = json!("e_foil");
    exact(&v, &[0], 66.0);
}

#[test]
fn holo_adds_10_mult() {
    let mut v = snap_json("AS");
    v["hand"][0]["ed"] = json!("e_holo");
    exact(&v, &[0], 176.0);
}

#[test]
fn polychrome_x1_5() {
    let mut v = snap_json("AS");
    v["hand"][0]["ed"] = json!("e_polychrome");
    exact(&v, &[0], 24.0);
}

#[test]
fn perma_bonus_adds_chips() {
    let mut v = snap_json("AS");
    v["hand"][0]["perma"] = json!({"bonus": 5});
    exact(&v, &[0], 21.0);
}

#[test]
fn flint_halves_base() {
    let mut v = snap_json("AS AH");
    v["blind"]["key"] = json!("bl_flint");
    exact(&v, &[0, 1], 27.0);
}

#[test]
fn flint_disabled_is_normal() {
    let mut v = snap_json("AS AH");
    v["blind"]["key"] = json!("bl_flint");
    v["blind"]["disabled"] = json!(true);
    exact(&v, &[0, 1], 64.0);
}

#[test]
fn plasma_balances() {
    let mut v = snap_json("AS AH");
    v["deck"]["back"] = json!("b_plasma");
    exact(&v, &[0, 1], 289.0);
}

#[test]
fn psychic_illegal_under_5() {
    let mut v = snap_json("AS");
    v["blind"]["key"] = json!("bl_psychic");
    let r = score(&to_snapshot(&v), &[0]);
    assert!(!r.legal);
    close(r.floor, 0.0);
    close(r.expected, 0.0);
    close(r.ceiling, 0.0);
}

#[test]
fn psychic_allows_5_cards() {
    let mut v = snap_json("AS AH 2D 3D 7C");
    v["blind"]["key"] = json!("bl_psychic");
    assert!(score(&to_snapshot(&v), &[0, 1, 2, 3, 4]).legal);
}

#[test]
fn eye_rejects_repeat_hand() {
    let mut v = snap_json("AS AH");
    v["blind"]["key"] = json!("bl_eye");
    v["levels"]["Pair"]["played_round"] = json!(1);
    assert!(!score(&to_snapshot(&v), &[0, 1]).legal);
    v["levels"]["Pair"]["played_round"] = json!(0);
    assert!(score(&to_snapshot(&v), &[0, 1]).legal);
}

#[test]
fn mouth_rejects_other_hand() {
    let mut v = snap_json("AS AH");
    v["blind"]["key"] = json!("bl_mouth");
    v["levels"]["Flush"]["played_round"] = json!(1);
    assert!(!score(&to_snapshot(&v), &[0, 1]).legal);
    v["levels"]["Flush"]["played_round"] = json!(0);
    v["levels"]["Pair"]["played_round"] = json!(1);
    assert!(score(&to_snapshot(&v), &[0, 1]).legal);
}

#[test]
fn arm_lowers_level_first() {
    let mut v = snap_json("AS AH");
    v["blind"]["key"] = json!("bl_arm");
    v["levels"]["Pair"] = json!({"level":2,"chips":25,"mult":3,"l_chips":15,"l_mult":1});
    exact(&v, &[0, 1], 64.0);
}

#[test]
fn arm_keeps_level_1() {
    let mut v = snap_json("AS AH");
    v["blind"]["key"] = json!("bl_arm");
    exact(&v, &[0, 1], 64.0);
}

#[test]
fn debuffed_card_adds_nothing() {
    let mut v = snap_json("AS");
    v["hand"][0]["debuff"] = json!(true);
    exact(&v, &[0], 5.0);
}

#[test]
fn steel_held_x1_5() {
    let mut v = snap_json("AS KH");
    v["hand"][1]["enh"] = json!("m_steel");
    exact(&v, &[0], 24.0);
}

#[test]
fn red_seal_retriggers_held_steel() {
    let mut v = snap_json("AS KH");
    v["hand"][1]["enh"] = json!("m_steel");
    v["hand"][1]["seal"] = json!("Red");
    exact(&v, &[0], 36.0);
}

#[test]
fn debuffed_held_steel_does_nothing() {
    let mut v = snap_json("AS KH");
    v["hand"][1]["enh"] = json!("m_steel");
    v["hand"][1]["debuff"] = json!(true);
    exact(&v, &[0], 16.0);
}

#[test]
fn blue_seal_gold_no_score_effect() {
    let mut v = snap_json("AS");
    v["hand"][0]["seal"] = json!("Blue");
    exact(&v, &[0], 16.0);
    v["hand"][0]["seal"] = json!("Gold");
    exact(&v, &[0], 16.0);
}

#[test]
fn observatory_x1_5_for_matching_planet() {
    let mut v = snap_json("AS");
    v["vouchers"] = json!(["v_observatory"]);
    v["consumables"] = json!([{"key": "c_pluto"}]);
    exact(&v, &[0], 24.0);
    v["consumables"] = json!([{"key": "c_mercury"}]);
    exact(&v, &[0], 16.0);
}

#[test]
fn infinite_score_serializes_naneinf() {
    let mut v = snap_json("AS");
    v["levels"]["High Card"]["chips"] = json!(1e308);
    v["levels"]["High Card"]["mult"] = json!(1e308);
    let r = score(&to_snapshot(&v), &[0]);
    assert!(r.expected.is_infinite());
    let out = r.to_json();
    assert_eq!(out["expected"], json!("naneinf"));
}

#[test]
fn nan_serializes_naneinf() {
    assert_eq!(mortemqol_engine::score::to_json_number(f64::NAN), json!("naneinf"));
    assert_eq!(mortemqol_engine::score::to_json_number(2.5), json!(2.5));
}

#[test]
fn hidden_held_card_flags_approximate() {
    let mut v = snap_json("AS KH");
    v["hand"][1] = json!({"id": 2, "hidden": true});
    let r = score(&to_snapshot(&v), &[0]);
    assert!(r.approximate.contains(&"hidden_card".to_string()));
    close(r.expected, 16.0);
}

#[test]
fn lobby_flag_approximate() {
    let mut v = snap_json("AS");
    v["lobby"] = json!(true);
    assert!(score(&to_snapshot(&v), &[0]).approximate.contains(&"lobby".to_string()));
}

#[test]
fn hook_flag_approximate() {
    let mut v = snap_json("AS");
    v["blind"]["key"] = json!("bl_hook");
    assert!(score(&to_snapshot(&v), &[0]).approximate.contains(&"hook".to_string()));
}

#[test]
fn unknown_enhancement_flags_approximate() {
    let mut v = snap_json("AS");
    v["hand"][0]["enh"] = json!("m_modded");
    let r = score(&to_snapshot(&v), &[0]);
    assert!(r.approximate.contains(&"unknown_enhancement:m_modded".to_string()));
    close(r.expected, 16.0);
}

#[test]
fn unknown_edition_and_seal_flag_approximate() {
    let mut v = snap_json("AS");
    v["hand"][0]["ed"] = json!("e_modded");
    v["hand"][0]["seal"] = json!("Modded");
    let r = score(&to_snapshot(&v), &[0]);
    assert!(r.approximate.contains(&"unknown_edition:e_modded".to_string()));
    assert!(r.approximate.contains(&"unknown_seal:Modded".to_string()));
}

#[test]
fn clean_snapshot_is_not_approximate() {
    assert!(score(&snap("AS AH"), &[0, 1]).approximate.is_empty());
}

#[test]
fn ffi_score_round_trip() {
    let v = json!({"snapshot": snap_json("AS AH KD"), "play": [1, 2]});
    let result = mortemqol_engine::score::score_request(v.to_string().as_bytes()).unwrap();
    let out: serde_json::Value = serde_json::from_slice(&result).unwrap();
    assert_eq!(out["hand"], json!("Pair"));
    assert_eq!(out["expected"], json!(64.0));
    assert_eq!(out["scored"], json!([1, 2]));
}

#[test]
fn ffi_score_unknown_card_id_is_error() {
    let v = json!({"snapshot": snap_json("AS"), "play": [99]});
    assert!(mortemqol_engine::score::score_request(v.to_string().as_bytes()).is_err());
}
