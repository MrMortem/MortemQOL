mod common;
use common::{close, j, number_jokers, play, score, snap_json, to_snapshot};
use serde_json::{json, Value};

fn ex(hand: &str, held: usize, jokers: Vec<Value>, want: f64) {
    let r = play(hand, held, jokers);
    close(r.floor, want);
    close(r.expected, want);
    close(r.ceiling, want);
}

fn edit(hand: &str, held: usize, jokers: Vec<Value>, f: impl Fn(&mut Value)) -> f64 {
    let mut v = snap_json(hand);
    v["jokers"] = Value::Array(number_jokers(jokers));
    f(&mut v);
    let s = to_snapshot(&v);
    let idx: Vec<usize> = (0..s.hand.len() - held).collect();
    score(&s, &idx).expected
}

fn joker4() -> Value {
    j("j_joker", json!({"mult": 4}))
}

#[test]
fn j_blueprint_copies_right() {
    let bp = || j("j_blueprint", json!({}));
    ex("AS", 0, vec![bp(), joker4()], 144.0);
    ex("AS", 0, vec![joker4(), bp()], 80.0);
}

#[test]
fn j_blueprint_copies_nothing_from_debuffed() {
    let mut d = joker4();
    d["debuff"] = json!(true);
    ex("AS", 0, vec![j("j_blueprint", json!({})), d], 16.0);
}

#[test]
fn j_brainstorm_copies_leftmost() {
    ex("AS", 0, vec![joker4(), j("j_brainstorm", json!({}))], 144.0);
}

#[test]
fn blueprint_chain_ending_in_itself_has_no_effect() {
    // Brainstorm copies the first joker, Blueprint, which copies Brainstorm.
    ex("AS", 0, vec![j("j_blueprint", json!({})), j("j_brainstorm", json!({}))], 16.0);
}

#[test]
fn blueprint_chain_reaches_real_joker() {
    let jokers = vec![j("j_blueprint", json!({})), j("j_blueprint", json!({})), joker4()];
    ex("AS", 0, jokers, 208.0);
}

#[test]
fn j_space_weighted_levelup() {
    let r = play("AS AH", 0, vec![j("j_space", json!({"extra": 4}))]);
    close(r.floor, 64.0);
    close(r.expected, 83.25);
    close(r.ceiling, 141.0);
    assert!(r.has_chance);
}

#[test]
fn j_hanging_chad_first_card() {
    ex("AS", 0, vec![j("j_hanging_chad", json!({"extra": 2}))], 38.0);
}

#[test]
fn j_photograph_card_order() {
    let photo = || vec![j("j_photograph", json!({"extra": 2}))];
    // Pair of kings, 30 chips. Mult King first: (2 + 4) X2. Plain King first: 2 X2 + 4.
    let first = edit("KS KH", 0, photo(), |v| v["hand"][0]["enh"] = json!("m_mult"));
    close(first, 360.0);
    let second = edit("KS KH", 0, photo(), |v| v["hand"][1]["enh"] = json!("m_mult"));
    close(second, 240.0);
}

#[test]
fn j_green_joker_counts_this_hand() {
    ex("AS", 0, vec![j("j_green_joker", json!({"mult": 0, "extra": {"hand_add": 1}}))], 32.0);
}

#[test]
fn j_ride_the_bus_builds_without_faces() {
    ex("AS", 0, vec![j("j_ride_the_bus", json!({"mult": 2, "extra": 1}))], 64.0);
}

#[test]
fn j_ride_the_bus_resets_on_face() {
    ex("KS", 0, vec![j("j_ride_the_bus", json!({"mult": 5, "extra": 1}))], 15.0);
}

#[test]
fn j_square_counts_four_card_play() {
    ex("2S 3H 7D 9C", 0, vec![j("j_square", json!({"extra": {"chips": 0, "chip_mod": 4}}))], 18.0);
}

#[test]
fn j_runner_on_straight() {
    ex(
        "2S 3H 4D 5C 6S",
        0,
        vec![j("j_runner", json!({"extra": {"chips": 0, "chip_mod": 15}}))],
        260.0,
    );
}

#[test]
fn j_trousers_on_two_pair() {
    ex("AS AH KS KH", 0, vec![j("j_trousers", json!({"mult": 0, "extra": 2}))], 248.0);
    ex("AS", 0, vec![j("j_trousers", json!({"mult": 0, "extra": 2}))], 16.0);
}

#[test]
fn j_obelisk_scales_when_not_most_played() {
    let o = || vec![j("j_obelisk", json!({"extra": 0.2, "x_mult": 1}))];
    let got = edit("AS AH", 0, o(), |v| {
        v["levels"]["Flush"]["played"] = json!(1);
        v["levels"]["Pair"]["played"] = json!(0);
    });
    close(got, 76.8);
}

#[test]
fn j_obelisk_resets_on_most_played() {
    let o = vec![j("j_obelisk", json!({"extra": 0.2, "x_mult": 3}))];
    close(edit("AS AH", 0, o, |_| {}), 64.0);
}

#[test]
fn j_vampire_strips_and_scales() {
    let v = || vec![j("j_vampire", json!({"extra": 0.1, "x_mult": 1}))];
    let got = edit("AS", 0, v(), |s| s["hand"][0]["enh"] = json!("m_bonus"));
    close(got, 17.6);
}

#[test]
fn j_vampire_skips_debuffed() {
    let v = vec![j("j_vampire", json!({"extra": 0.1, "x_mult": 1}))];
    let got = edit("AS", 0, v, |s| {
        s["hand"][0]["enh"] = json!("m_bonus");
        s["hand"][0]["debuff"] = json!(true);
    });
    close(got, 5.0);
}

#[test]
fn j_midas_mask_turns_faces_gold() {
    let got = edit("KS", 0, vec![j("j_midas_mask", json!({}))], |s| {
        s["hand"][0]["enh"] = json!("m_glass")
    });
    close(got, 15.0);
}

#[test]
fn j_hiker_applies_on_retrigger() {
    let got = edit("AS", 0, vec![j("j_hiker", json!({"extra": 5}))], |s| {
        s["hand"][0]["seal"] = json!("Red")
    });
    close(got, 32.0);
}

#[test]
fn j_mime_retriggers_held_steel() {
    let got = edit("AS KH", 1, vec![j("j_mime", json!({"extra": 1}))], |s| {
        s["hand"][1]["enh"] = json!("m_steel")
    });
    close(got, 36.0);
}

#[test]
fn j_dusk_on_last_hand() {
    let d = || vec![j("j_dusk", json!({"extra": 1}))];
    close(edit("AS", 0, d(), |s| s["round"]["hands_left"] = json!(1)), 27.0);
    close(edit("AS", 0, d(), |s| s["round"]["hands_left"] = json!(2)), 16.0);
}

#[test]
fn j_sock_and_buskin_faces() {
    ex("KS", 0, vec![j("j_sock_and_buskin", json!({"extra": 1}))], 25.0);
    ex("AS", 0, vec![j("j_sock_and_buskin", json!({"extra": 1}))], 16.0);
}

#[test]
fn j_hack_low_ranks() {
    ex("2S", 0, vec![j("j_hack", json!({"extra": 1}))], 9.0);
    ex("9S", 0, vec![j("j_hack", json!({"extra": 1}))], 14.0);
}

#[test]
fn j_selzer_retriggers_all() {
    ex("AS", 0, vec![j("j_selzer", json!({"extra": 10}))], 27.0);
}

#[test]
fn j_ice_cream_chips() {
    ex("AS", 0, vec![j("j_ice_cream", json!({"extra": {"chips": 100, "chip_mod": 5}}))], 116.0);
}

#[test]
fn j_lucky_cat_scales_on_trigger() {
    let mut v = snap_json("AS");
    v["jokers"] = json!([j("j_lucky_cat", json!({"extra": 0.25, "x_mult": 1}))]);
    v["hand"][0]["enh"] = json!("m_lucky");
    let r = score(&to_snapshot(&v), &[0]);
    close(r.floor, 16.0);
    close(r.ceiling, 16.0 * 21.0 * 1.25);
    let p: f64 = 1.0 - (0.8 * (14.0 / 15.0));
    close(r.expected, 16.0 * 5.0 * (1.0 + 0.25 * p));
}

#[test]
fn j_wee_scales_on_two() {
    ex("2S", 0, vec![j("j_wee", json!({"extra": {"chips": 0, "chip_mod": 8}}))], 15.0);
}

#[test]
fn j_todo_list_money_feeds_bull() {
    let jokers = vec![
        j("j_todo_list", json!({"extra": {"dollars": 4}, "to_do_poker_hand": "High Card"})),
        j("j_bull", json!({"extra": 2})),
    ];
    ex("AS", 0, jokers, 32.0);
}

#[test]
fn j_dna_copy_counts_as_held() {
    let d = || vec![j("j_dna", json!({}))];
    let steel = |s: &mut Value| s["hand"][0]["enh"] = json!("m_steel");
    close(edit("AS", 0, d(), steel), 24.0);
    close(
        edit("AS", 0, d(), |s| {
            steel(s);
            s["round"]["round_hands"] = json!(1);
        }),
        16.0,
    );
}

#[test]
fn j_bull_reads_money_gained_this_hand() {
    let got = edit("AS", 0, vec![j("j_bull", json!({"extra": 2}))], |s| {
        s["hand"][0]["seal"] = json!("Gold")
    });
    close(got, 30.0);
}

#[test]
fn j_bootstraps_reads_money_gained_this_hand() {
    let b = vec![j("j_bootstraps", json!({"extra": {"mult": 2, "dollars": 5}}))];
    let got = edit("AS", 0, b, |s| s["hand"][0]["seal"] = json!("Gold"));
    close(got, 48.0);
}

#[test]
fn ox_zeroes_money_before_bull() {
    let got = edit("AS", 0, vec![j("j_bull", json!({"extra": 2}))], |s| {
        s["blind"]["key"] = json!("bl_ox");
        s["round"]["most_played"] = json!("High Card");
    });
    close(got, 16.0);
}

#[test]
fn copied_scaler_does_not_scale_twice() {
    let jokers = vec![
        j("j_blueprint", json!({})),
        j("j_green_joker", json!({"mult": 0, "extra": {"hand_add": 1}})),
    ];
    ex("AS", 0, jokers, 48.0);
}

#[test]
fn copied_retrigger_counts() {
    let jokers = vec![j("j_blueprint", json!({})), j("j_selzer", json!({"extra": 10}))];
    ex("AS", 0, jokers, 38.0);
}

#[test]
fn copied_loyalty_reads_stale_counter_when_left_of_source() {
    let l = j(
        "j_loyalty_card",
        json!({"extra": {"Xmult": 4, "every": 5}, "hands_played_at_create": 0, "loyalty_remaining": 0}),
    );
    let got = edit("AS", 0, vec![j("j_blueprint", json!({})), l], |v| {
        v["round"]["hands_played"] = json!(5)
    });
    close(got, 64.0);
}
