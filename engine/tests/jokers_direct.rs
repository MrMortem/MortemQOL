mod common;
use common::{close, j, number_jokers, play, score, snap_json, to_snapshot};
use serde_json::{json, Value};

fn ex(hand: &str, held: usize, jokers: Vec<Value>, want: f64) {
    let r = play(hand, held, jokers);
    close(r.floor, want);
    close(r.expected, want);
    close(r.ceiling, want);
}

/// Runs with edits to the snapshot.
fn edit(hand: &str, held: usize, jokers: Vec<Value>, f: impl Fn(&mut Value)) -> f64 {
    let mut v = snap_json(hand);
    v["jokers"] = Value::Array(number_jokers(jokers));
    f(&mut v);
    let s = to_snapshot(&v);
    let idx: Vec<usize> = (0..s.hand.len() - held).collect();
    score(&s, &idx).expected
}

#[test]
fn j_joker_adds_mult() {
    ex("AS", 0, vec![j("j_joker", json!({"mult": 4}))], 80.0);
}

#[test]
fn j_joker_then_cavendish_order() {
    let joker = j("j_joker", json!({"mult": 4}));
    let cav = j("j_cavendish", json!({"extra": {"Xmult": 3}}));
    ex("AS", 0, vec![joker.clone(), cav.clone()], 240.0);
    ex("AS", 0, vec![cav, joker], 112.0);
}

fn suit_joker(key: &str, suit: &str) -> Value {
    j(key, json!({"extra": {"s_mult": 3, "suit": suit}}))
}

#[test]
fn j_greedy_joker_diamonds() {
    ex("AD", 0, vec![suit_joker("j_greedy_joker", "Diamonds")], 64.0);
    ex("AS", 0, vec![suit_joker("j_greedy_joker", "Diamonds")], 16.0);
}

#[test]
fn j_lusty_joker_hearts() {
    ex("AH", 0, vec![suit_joker("j_lusty_joker", "Hearts")], 64.0);
}

#[test]
fn j_wrathful_joker_spades() {
    ex("AS", 0, vec![suit_joker("j_wrathful_joker", "Spades")], 64.0);
}

#[test]
fn j_gluttenous_joker_clubs() {
    ex("AC", 0, vec![suit_joker("j_gluttenous_joker", "Clubs")], 64.0);
}

fn tm(key: &str, t: i32, ty: &str) -> Value {
    j(key, json!({"t_mult": t, "type": ty}))
}

fn tc(key: &str, t: i32, ty: &str) -> Value {
    j(key, json!({"t_chips": t, "type": ty}))
}

fn tx(key: &str, x: f64, ty: &str) -> Value {
    j(key, json!({"x_mult": x, "type": ty}))
}

#[test]
fn j_jolly_pair_contained() {
    ex("AS AH", 0, vec![tm("j_jolly", 8, "Pair")], 320.0);
    ex("AS", 0, vec![tm("j_jolly", 8, "Pair")], 16.0);
    // A Full House contains a Pair. 40 + 33 + 20 chips, 4 + 8 mult.
    ex("AS AH AD KS KH", 0, vec![tm("j_jolly", 8, "Pair")], 1116.0);
}

#[test]
fn j_zany_three() {
    ex("AS AH AD", 0, vec![tm("j_zany", 12, "Three of a Kind")], 945.0);
}

#[test]
fn j_mad_two_pair() {
    ex("AS AH KS KH", 0, vec![tm("j_mad", 10, "Two Pair")], 744.0);
}

#[test]
fn j_crazy_straight() {
    ex("2S 3H 4D 5C 6S", 0, vec![tm("j_crazy", 12, "Straight")], 800.0);
}

#[test]
fn j_droll_flush() {
    ex("2S 9S 4S 5S KS", 0, vec![tm("j_droll", 10, "Flush")], 910.0);
}

#[test]
fn j_sly_pair() {
    ex("AS AH", 0, vec![tc("j_sly", 50, "Pair")], 164.0);
}

#[test]
fn j_wily_three() {
    ex("AS AH AD", 0, vec![tc("j_wily", 100, "Three of a Kind")], 489.0);
}

#[test]
fn j_clever_two_pair() {
    ex("AS AH KS KH", 0, vec![tc("j_clever", 80, "Two Pair")], 284.0);
}

#[test]
fn j_devious_straight() {
    ex("2S 3H 4D 5C 6S", 0, vec![tc("j_devious", 100, "Straight")], 600.0);
}

#[test]
fn j_crafty_flush() {
    ex("2S 9S 4S 5S KS", 0, vec![tc("j_crafty", 80, "Flush")], 580.0);
}

#[test]
fn j_half_three_or_fewer_cards() {
    let half = || j("j_half", json!({"extra": {"mult": 20, "size": 3}}));
    ex("AS", 0, vec![half()], 336.0);
    ex("2S 3H 7D 9C", 0, vec![half()], 14.0);
}

#[test]
fn j_stencil_uses_x_mult() {
    ex("AS", 0, vec![j("j_stencil", json!({"x_mult": 2}))], 32.0);
}

#[test]
fn j_four_fingers_enables_short_flush() {
    let r = play("2S 9S 4S KS 7H", 0, vec![j("j_four_fingers", json!({}))]);
    assert_eq!(r.hand, "Flush");
}

#[test]
fn j_shortcut_enables_gapped_straight() {
    assert_eq!(play("2S 4H 6D 8C 10S", 0, vec![j("j_shortcut", json!({}))]).hand, "Straight");
}

#[test]
fn j_smeared_merges_suits() {
    assert_eq!(play("2H 9D 4H KD 7H", 0, vec![j("j_smeared", json!({}))]).hand, "Flush");
}

#[test]
fn j_splash_scores_kicker() {
    ex("AS AH KD", 0, vec![j("j_splash", json!({}))], 84.0);
}

#[test]
fn j_pareidolia_makes_scary_face_trigger() {
    let jokers = vec![j("j_pareidolia", json!({})), j("j_scary_face", json!({"extra": 30}))];
    ex("2S", 0, jokers, 37.0);
}

#[test]
fn j_ceremonial_mult() {
    ex("AS", 0, vec![j("j_ceremonial", json!({"mult": 6}))], 112.0);
}

#[test]
fn j_banner_per_discard() {
    ex("AS", 0, vec![j("j_banner", json!({"extra": 30}))], 106.0);
}

#[test]
fn j_mystic_summit_at_zero_discards() {
    let m = || vec![j("j_mystic_summit", json!({"extra": {"mult": 15, "d_remaining": 0}}))];
    close(edit("AS", 0, m(), |v| v["round"]["discards_left"] = json!(0)), 256.0);
    close(edit("AS", 0, m(), |_| {}), 16.0);
}

#[test]
fn j_loyalty_card_every_sixth() {
    let l = || {
        vec![j(
            "j_loyalty_card",
            json!({"extra": {"Xmult": 4, "every": 5}, "hands_played_at_create": 0}),
        )]
    };
    close(edit("AS", 0, l(), |v| v["round"]["hands_played"] = json!(5)), 64.0);
    close(edit("AS", 0, l(), |v| v["round"]["hands_played"] = json!(4)), 16.0);
}

#[test]
fn j_misprint_range() {
    let r = play("AS", 0, vec![j("j_misprint", json!({"extra": {"min": 0, "max": 23}}))]);
    close(r.floor, 16.0);
    close(r.expected, 200.0);
    close(r.ceiling, 384.0);
    assert!(r.has_chance);
}

#[test]
fn j_raised_fist_lowest_held() {
    ex("AS 5H 9D", 2, vec![j("j_raised_fist", json!({}))], 176.0);
}

#[test]
fn j_raised_fist_debuffed_lowest_gives_nothing() {
    let got = edit("AS 5H 9D", 2, vec![j("j_raised_fist", json!({}))], |v| {
        v["hand"][1]["debuff"] = json!(true)
    });
    close(got, 16.0);
}

#[test]
fn j_fibonacci_ace() {
    ex("AS", 0, vec![j("j_fibonacci", json!({"extra": 8}))], 144.0);
    ex("KS", 0, vec![j("j_fibonacci", json!({"extra": 8}))], 15.0);
}

#[test]
fn j_steel_joker_tally() {
    ex("AS", 0, vec![j("j_steel_joker", json!({"extra": 0.2, "steel_tally": 2}))], 22.4);
}

#[test]
fn j_scary_face_on_face() {
    ex("KS", 0, vec![j("j_scary_face", json!({"extra": 30}))], 45.0);
}

#[test]
fn j_abstract_per_joker() {
    ex("AS", 0, vec![j("j_abstract", json!({"extra": 3}))], 64.0);
}

#[test]
fn j_gros_michel_mult() {
    ex("AS", 0, vec![j("j_gros_michel", json!({"extra": {"mult": 15, "odds": 6}}))], 256.0);
}

#[test]
fn j_even_steven_even() {
    ex("2S", 0, vec![j("j_even_steven", json!({"extra": 4}))], 35.0);
    ex("KS", 0, vec![j("j_even_steven", json!({"extra": 4}))], 15.0);
}

#[test]
fn j_odd_todd_odd_and_ace() {
    ex("3S", 0, vec![j("j_odd_todd", json!({"extra": 31}))], 39.0);
    ex("AS", 0, vec![j("j_odd_todd", json!({"extra": 31}))], 47.0);
}

#[test]
fn j_scholar_ace() {
    ex("AS", 0, vec![j("j_scholar", json!({"extra": {"mult": 4, "chips": 20}}))], 180.0);
}

#[test]
fn j_business_no_score_change() {
    ex("KS", 0, vec![j("j_business", json!({"extra": 2}))], 15.0);
}

#[test]
fn j_supernova_counts_this_play() {
    let got = edit("AS", 0, vec![j("j_supernova", json!({"extra": 1}))], |v| {
        v["levels"]["High Card"]["played"] = json!(3)
    });
    close(got, 80.0);
}

#[test]
fn j_blackboard_all_black_held() {
    ex("AS 2C", 1, vec![j("j_blackboard", json!({"extra": 3}))], 48.0);
    ex("AS 2H", 1, vec![j("j_blackboard", json!({"extra": 3}))], 16.0);
}

#[test]
fn j_blue_joker_per_deck_card() {
    ex("AS", 0, vec![j("j_blue_joker", json!({"extra": 2}))], 104.0);
}

#[test]
fn j_constellation_x_mult() {
    ex("AS", 0, vec![tx("j_constellation", 1.5, "")], 24.0);
}

#[test]
fn j_hologram_x_mult() {
    ex("AS", 0, vec![tx("j_hologram", 1.5, "")], 24.0);
}

#[test]
fn j_madness_x_mult() {
    ex("AS", 0, vec![tx("j_madness", 2.0, "")], 32.0);
}

#[test]
fn j_throwback_x_mult() {
    ex("AS", 0, vec![tx("j_throwback", 1.5, "")], 24.0);
}

#[test]
fn j_glass_x_mult() {
    ex("AS", 0, vec![tx("j_glass", 1.75, "")], 28.0);
}

#[test]
fn j_campfire_x_mult() {
    ex("AS", 0, vec![tx("j_campfire", 1.25, "")], 20.0);
}

#[test]
fn j_hit_the_road_x_mult() {
    ex("AS", 0, vec![tx("j_hit_the_road", 2.0, "")], 32.0);
}

#[test]
fn j_ramen_x_mult() {
    ex("AS", 0, vec![tx("j_ramen", 2.0, "")], 32.0);
}

#[test]
fn j_yorick_x_mult() {
    ex("AS", 0, vec![tx("j_yorick", 2.0, "")], 32.0);
}

#[test]
fn j_duo_needs_pair() {
    ex("AS AH", 0, vec![tx("j_duo", 2.0, "Pair")], 128.0);
    ex("AS", 0, vec![tx("j_duo", 2.0, "Pair")], 16.0);
}

#[test]
fn j_trio_needs_three() {
    ex("AS AH AD", 0, vec![tx("j_trio", 3.0, "Three of a Kind")], 567.0);
}

#[test]
fn j_family_needs_four() {
    // 60 + 44 chips, 7 mult, X4.
    ex("AS AH AD AC", 0, vec![tx("j_family", 4.0, "Four of a Kind")], 2912.0);
}

#[test]
fn j_order_needs_straight() {
    ex("2S 3H 4D 5C 6S", 0, vec![tx("j_order", 3.0, "Straight")], 600.0);
}

#[test]
fn j_tribe_needs_flush() {
    ex("2S 9S 4S 5S KS", 0, vec![tx("j_tribe", 2.0, "Flush")], 520.0);
}

#[test]
fn j_cavendish_x3() {
    ex("AS", 0, vec![j("j_cavendish", json!({"extra": {"Xmult": 3}}))], 48.0);
}

#[test]
fn j_card_sharp_on_repeat() {
    let c = || vec![j("j_card_sharp", json!({"extra": {"Xmult": 3}}))];
    close(edit("AS", 0, c(), |v| v["levels"]["High Card"]["played_round"] = json!(1)), 48.0);
    close(edit("AS", 0, c(), |_| {}), 16.0);
}

#[test]
fn j_red_card_mult() {
    ex("AS", 0, vec![j("j_red_card", json!({"mult": 6}))], 112.0);
}

#[test]
fn j_baron_held_king() {
    ex("AS KH", 1, vec![j("j_baron", json!({"extra": 1.5}))], 24.0);
}

#[test]
fn j_photograph_first_face() {
    ex("KS", 0, vec![j("j_photograph", json!({"extra": 2}))], 30.0);
    ex("AS", 0, vec![j("j_photograph", json!({"extra": 2}))], 16.0);
}

#[test]
fn j_erosion_missing_cards() {
    let got = edit("AS", 0, vec![j("j_erosion", json!({"extra": 4}))], |v| {
        v["deck"]["starting"] = json!(52);
        v["deck"]["total"] = json!(50);
    });
    close(got, 144.0);
}

#[test]
fn j_reserved_parking_no_score_change() {
    ex(
        "AS KH",
        1,
        vec![j("j_reserved_parking", json!({"extra": {"odds": 2, "dollars": 1}}))],
        16.0,
    );
}

#[test]
fn j_fortune_teller_tarots_used() {
    let got = edit("AS", 0, vec![j("j_fortune_teller", json!({}))], |v| {
        v["round"]["tarot_used"] = json!(3)
    });
    close(got, 64.0);
}

#[test]
fn j_stone_tally() {
    ex("AS", 0, vec![j("j_stone", json!({"extra": 25, "stone_tally": 2}))], 66.0);
}

#[test]
fn j_baseball_x_per_uncommon() {
    let mut uncommon = j("j_fibonacci", json!({"extra": 8}));
    uncommon["rarity"] = json!(2);
    let mut ball = j("j_baseball", json!({"extra": 1.5}));
    ball["rarity"] = json!(3);
    ex("AS", 0, vec![ball.clone(), uncommon], 216.0);
    ex("AS", 0, vec![ball], 16.0);
}

#[test]
fn j_bull_per_dollar() {
    ex("AS", 0, vec![j("j_bull", json!({"extra": 2}))], 24.0);
}

#[test]
fn j_flash_mult() {
    ex("AS", 0, vec![j("j_flash", json!({"mult": 4}))], 80.0);
}

#[test]
fn j_popcorn_mult() {
    ex("AS", 0, vec![j("j_popcorn", json!({"mult": 20}))], 336.0);
}

#[test]
fn j_ancient_current_suit() {
    let a = || vec![j("j_ancient", json!({"extra": 1.5}))];
    let hearts = |v: &mut Value| v["round"]["ancient_suit"] = json!("Hearts");
    close(edit("AH", 0, a(), hearts), 24.0);
    close(edit("AS", 0, a(), hearts), 16.0);
}

#[test]
fn j_walkie_talkie_tens_and_fours() {
    let w = || vec![j("j_walkie_talkie", json!({"extra": {"chips": 10, "mult": 4}}))];
    ex("4S", 0, w(), 95.0);
    ex("10S", 0, w(), 125.0);
}

#[test]
fn j_castle_chips() {
    ex("AS", 0, vec![j("j_castle", json!({"extra": {"chips": 30, "chip_mod": 3}}))], 46.0);
}

#[test]
fn j_smiley_on_face() {
    ex("KS", 0, vec![j("j_smiley", json!({"extra": 5}))], 90.0);
}

#[test]
fn j_ticket_no_score_change() {
    let got = edit("AS", 0, vec![j("j_ticket", json!({"extra": 4}))], |v| {
        v["hand"][0]["enh"] = json!("m_gold")
    });
    close(got, 16.0);
}

#[test]
fn j_acrobat_on_last_hand() {
    let a = || vec![j("j_acrobat", json!({"extra": 3}))];
    close(edit("AS", 0, a(), |v| v["round"]["hands_left"] = json!(1)), 48.0);
    close(edit("AS", 0, a(), |v| v["round"]["hands_left"] = json!(2)), 16.0);
}

#[test]
fn j_swashbuckler_mult() {
    ex("AS", 0, vec![j("j_swashbuckler", json!({"mult": 5}))], 96.0);
}

#[test]
fn j_rough_gem_feeds_bull() {
    let jokers = vec![j("j_rough_gem", json!({"extra": 1})), j("j_bull", json!({"extra": 2}))];
    ex("AD", 0, jokers, 26.0);
}

#[test]
fn j_bloodstone_range() {
    let r = play("AH", 0, vec![j("j_bloodstone", json!({"extra": {"odds": 2, "Xmult": 1.5}}))]);
    close(r.floor, 16.0);
    close(r.expected, 20.0);
    close(r.ceiling, 24.0);
}

#[test]
fn j_arrowhead_spades() {
    ex("AS", 0, vec![j("j_arrowhead", json!({"extra": 50}))], 66.0);
}

#[test]
fn j_onyx_agate_clubs() {
    ex("AC", 0, vec![j("j_onyx_agate", json!({"extra": 7}))], 128.0);
}

#[test]
fn j_flower_pot_all_suits() {
    ex("2S 3H 4D 5C 6S", 0, vec![j("j_flower_pot", json!({"extra": 3}))], 600.0);
    ex("AS AH", 0, vec![j("j_flower_pot", json!({"extra": 3}))], 64.0);
}

#[test]
fn j_oops_doubles_lucky() {
    let got =
        edit("AS", 0, vec![j("j_oops", json!({}))], |v| v["hand"][0]["enh"] = json!("m_lucky"));
    close(got, 144.0);
}

#[test]
fn j_idol_current_card() {
    let i = || vec![j("j_idol", json!({"extra": 2}))];
    let set = |v: &mut Value| v["round"]["idol"] = json!({"rank": 14, "suit": "Spades"});
    close(edit("AS", 0, i(), set), 32.0);
    close(edit("KS", 0, i(), set), 15.0);
}

#[test]
fn j_seeing_double_club_and_other() {
    ex("AC AS", 0, vec![j("j_seeing_double", json!({"extra": 2}))], 128.0);
    ex("AS AH", 0, vec![j("j_seeing_double", json!({"extra": 2}))], 64.0);
}

#[test]
fn j_matador_money_after_blind_triggers() {
    let jokers = vec![j("j_matador", json!({"extra": 8})), j("j_bull", json!({"extra": 2}))];
    let got = edit("AS", 0, jokers, |v| v["hand"][0]["debuff"] = json!(true));
    close(got, 29.0);
}

#[test]
fn j_stuntman_chips() {
    ex("AS", 0, vec![j("j_stuntman", json!({"extra": {"chip_mod": 250, "h_size": 2}}))], 266.0);
}

#[test]
fn j_shoot_the_moon_held_queen() {
    ex("AS QH", 1, vec![j("j_shoot_the_moon", json!({"extra": 13}))], 224.0);
}

#[test]
fn j_drivers_license_16_enhanced() {
    ex("AS", 0, vec![j("j_drivers_license", json!({"extra": 3, "driver_tally": 16}))], 48.0);
    ex("AS", 0, vec![j("j_drivers_license", json!({"extra": 3, "driver_tally": 15}))], 16.0);
}

#[test]
fn j_bootstraps_per_five_dollars() {
    let got =
        edit("AS", 0, vec![j("j_bootstraps", json!({"extra": {"mult": 2, "dollars": 5}}))], |v| {
            v["round"]["dollars"] = json!(10)
        });
    close(got, 80.0);
}

#[test]
fn j_caino_x_mult() {
    ex("AS", 0, vec![j("j_caino", json!({"caino_xmult": 3}))], 48.0);
}

#[test]
fn j_triboulet_queens_and_kings() {
    ex("QS", 0, vec![j("j_triboulet", json!({"extra": 2}))], 30.0);
}

#[test]
fn debuffed_joker_has_no_effect() {
    let mut d = j("j_joker", json!({"mult": 4}));
    d["debuff"] = json!(true);
    ex("AS", 0, vec![d], 16.0);
}

#[test]
fn polychrome_joker_x1_5_after_its_effect() {
    let mut p = j("j_joker", json!({"mult": 4}));
    p["ed"] = json!("e_polychrome");
    ex("AS", 0, vec![p], 120.0);
}

#[test]
fn foil_joker_chips_before_its_effect() {
    let mut p = j("j_joker", json!({"mult": 4}));
    p["ed"] = json!("e_foil");
    ex("AS", 0, vec![p], 330.0);
}

#[test]
fn holo_joker_mult_before_its_effect() {
    let mut p = j("j_joker", json!({"mult": 4}));
    p["ed"] = json!("e_holo");
    ex("AS", 0, vec![p], 240.0);
}

#[test]
fn debuffed_rule_joker_does_not_apply() {
    let mut d = j("j_four_fingers", json!({}));
    d["debuff"] = json!(true);
    assert_eq!(play("2S 9S 4S KS 7H", 0, vec![d]).hand, "High Card");
}

#[test]
fn unknown_joker_no_effect_flagged() {
    let r = play("AS", 0, vec![j("j_modded", json!({"mult": 99}))]);
    assert!(r.approximate.contains(&"unknown_joker:j_modded".to_string()));
    close(r.expected, 16.0);
}

#[test]
fn hidden_joker_flagged() {
    let r = play("AS", 0, vec![json!({"id": 5, "hidden": true})]);
    assert!(r.approximate.contains(&"hidden_joker".to_string()));
    close(r.expected, 16.0);
}

#[test]
fn modeled_joker_is_not_flagged() {
    assert!(play("AS", 0, vec![j("j_joker", json!({"mult": 4}))]).approximate.is_empty());
}
