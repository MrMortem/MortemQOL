//! Vanilla joker effects, following Card:calculate_joker. Values come from the joker's
//! `ability` table in the snapshot; the defaults are the game's own.
use crate::hand::{self, Rules};
use crate::score::{JState, ScoreCtx};
use crate::snapshot::{Card, Joker, Snapshot};

const VANILLA: [&str; 150] = [
    "j_joker",
    "j_greedy_joker",
    "j_lusty_joker",
    "j_wrathful_joker",
    "j_gluttenous_joker",
    "j_jolly",
    "j_zany",
    "j_mad",
    "j_crazy",
    "j_droll",
    "j_sly",
    "j_wily",
    "j_clever",
    "j_devious",
    "j_crafty",
    "j_half",
    "j_stencil",
    "j_four_fingers",
    "j_mime",
    "j_credit_card",
    "j_ceremonial",
    "j_banner",
    "j_mystic_summit",
    "j_marble",
    "j_loyalty_card",
    "j_8_ball",
    "j_misprint",
    "j_dusk",
    "j_raised_fist",
    "j_chaos",
    "j_fibonacci",
    "j_steel_joker",
    "j_scary_face",
    "j_abstract",
    "j_delayed_grat",
    "j_hack",
    "j_pareidolia",
    "j_gros_michel",
    "j_even_steven",
    "j_odd_todd",
    "j_scholar",
    "j_business",
    "j_supernova",
    "j_ride_the_bus",
    "j_space",
    "j_egg",
    "j_burglar",
    "j_blackboard",
    "j_runner",
    "j_ice_cream",
    "j_dna",
    "j_splash",
    "j_blue_joker",
    "j_sixth_sense",
    "j_constellation",
    "j_hiker",
    "j_faceless",
    "j_green_joker",
    "j_superposition",
    "j_todo_list",
    "j_cavendish",
    "j_card_sharp",
    "j_red_card",
    "j_madness",
    "j_square",
    "j_seance",
    "j_riff_raff",
    "j_vampire",
    "j_shortcut",
    "j_hologram",
    "j_vagabond",
    "j_baron",
    "j_cloud_9",
    "j_rocket",
    "j_obelisk",
    "j_midas_mask",
    "j_luchador",
    "j_photograph",
    "j_gift",
    "j_turtle_bean",
    "j_erosion",
    "j_reserved_parking",
    "j_mail",
    "j_to_the_moon",
    "j_hallucination",
    "j_fortune_teller",
    "j_juggler",
    "j_drunkard",
    "j_stone",
    "j_golden",
    "j_lucky_cat",
    "j_baseball",
    "j_bull",
    "j_diet_cola",
    "j_trading",
    "j_flash",
    "j_popcorn",
    "j_trousers",
    "j_ancient",
    "j_ramen",
    "j_walkie_talkie",
    "j_selzer",
    "j_castle",
    "j_smiley",
    "j_campfire",
    "j_ticket",
    "j_mr_bones",
    "j_acrobat",
    "j_sock_and_buskin",
    "j_swashbuckler",
    "j_troubadour",
    "j_certificate",
    "j_smeared",
    "j_throwback",
    "j_hanging_chad",
    "j_rough_gem",
    "j_bloodstone",
    "j_arrowhead",
    "j_onyx_agate",
    "j_glass",
    "j_ring_master",
    "j_flower_pot",
    "j_blueprint",
    "j_wee",
    "j_merry_andy",
    "j_oops",
    "j_idol",
    "j_seeing_double",
    "j_matador",
    "j_hit_the_road",
    "j_duo",
    "j_trio",
    "j_family",
    "j_order",
    "j_tribe",
    "j_stuntman",
    "j_invisible",
    "j_brainstorm",
    "j_satellite",
    "j_shoot_the_moon",
    "j_drivers_license",
    "j_cartomancer",
    "j_astronomer",
    "j_burnt",
    "j_bootstraps",
    "j_caino",
    "j_triboulet",
    "j_yorick",
    "j_chicot",
    "j_perkeo",
];

pub fn is_known(key: &str) -> bool {
    VANILLA.contains(&key)
}

/// Rule-changing jokers are found by find_joker, which ignores debuffed ones.
pub fn rules(s: &Snapshot) -> Rules {
    let has = |key: &str| s.jokers.iter().any(|j| j.is(key) && !j.debuff);
    Rules {
        four_fingers: has("j_four_fingers"),
        shortcut: has("j_shortcut"),
        smeared: has("j_smeared"),
        pareidolia: has("j_pareidolia"),
        splash: has("j_splash"),
    }
}

pub fn initial_state(s: &Snapshot) -> Vec<JState> {
    s.jokers
        .iter()
        .map(|j| JState {
            mult: j.num("mult", 0.0),
            x_mult: j.num("x_mult", 1.0),
            chips: j.extra_num("chips", 0.0),
        })
        .collect()
}

/// Space Joker's `1 in N`, with the default from the game.
pub fn space_odds(s: &Snapshot) -> f64 {
    s.jokers.iter().find(|j| j.is("j_space")).map_or(4.0, |j| j.extra(4.0))
}

/// The joker whose effect the joker at `pos` produces, and whether it is copied.
/// Follows SMODS.blueprint_effect: copying stops at a debuffed joker, itself, or a loop.
fn resolve(s: &Snapshot, order: &[usize], pos: usize) -> Option<(usize, bool)> {
    let mut current = pos;
    let mut copied = false;
    for _ in 0..=order.len() {
        let joker = &s.jokers[order[current]];
        if joker.debuff || joker.hidden {
            return None;
        }
        let next = match joker.key.as_deref() {
            Some("j_blueprint") => current + 1,
            Some("j_brainstorm") => 0,
            _ => return Some((order[current], copied)),
        };
        if next >= order.len() || next == current {
            return None;
        }
        current = next;
        copied = true;
    }
    None
}

fn resolved_jokers(s: &Snapshot, order: &[usize]) -> Vec<(usize, bool)> {
    (0..order.len()).filter_map(|pos| resolve(s, order, pos)).collect()
}

/// Money from context.debuffed_hand: only Matador pays on a blocked hand.
pub fn debuffed_hand_money(s: &Snapshot, order: &[usize]) -> f64 {
    resolved_jokers(s, order)
        .iter()
        .filter(|(t, _)| s.jokers[*t].is("j_matador"))
        .map(|(t, _)| s.jokers[*t].extra(8.0))
        .sum()
}

pub fn space_triggers(s: &Snapshot, order: &[usize]) -> u32 {
    resolved_jokers(s, order).iter().filter(|(t, _)| s.jokers[*t].is("j_space")).count() as u32
}

/// context.before, in joker order. Scaling here counts toward this hand.
pub fn before(ctx: &mut ScoreCtx) {
    let s = ctx.s;
    let det = ctx.det;
    for (t, copied) in resolved_jokers(s, ctx.order) {
        let j = &s.jokers[t];
        match j.key.as_deref().unwrap_or("") {
            "j_trousers" if !copied && (det.has("Two Pair") || det.has("Full House")) => {
                ctx.jstate[t].mult += j.extra(2.0)
            }
            "j_square" if !copied && ctx.played.len() == 4 => {
                ctx.jstate[t].chips += j.extra_num("chip_mod", 4.0)
            }
            "j_runner" if !copied && det.has("Straight") => {
                ctx.jstate[t].chips += j.extra_num("chip_mod", 15.0)
            }
            "j_midas_mask" if !copied => {
                for &i in &det.scoring {
                    if hand::is_face(&ctx.played[i], &ctx.rules) {
                        ctx.played[i].to_mut().enh = Some("m_gold".to_string());
                    }
                }
            }
            "j_vampire" if !copied => {
                let mut count = 0.0;
                for &i in &det.scoring {
                    if ctx.played[i].enh.is_some() && !ctx.played[i].debuff {
                        ctx.played[i].to_mut().enh = None;
                        count += 1.0;
                    }
                }
                ctx.jstate[t].x_mult += j.extra(0.1) * count;
            }
            "j_todo_list"
                if j.ab.get("to_do_poker_hand").and_then(|v| v.as_str()) == Some(det.name) =>
            {
                ctx.dollars += j.extra_num("dollars", 4.0)
            }
            "j_dna" if s.round.round_hands == 0.0 && ctx.played.len() == 1 => {
                let copy = ctx.played[0].clone();
                ctx.held.push(copy);
                ctx.cards_added += 1.0;
                // Adding a playing card scales Hologram immediately.
                for (index, joker) in s.jokers.iter().enumerate() {
                    if joker.is("j_hologram") && !joker.debuff && !joker.hidden {
                        ctx.jstate[index].x_mult += joker.extra(0.25);
                    }
                }
            }
            "j_ride_the_bus" if !copied => {
                if det.scoring.iter().any(|&i| hand::is_face(&ctx.played[i], &ctx.rules)) {
                    ctx.jstate[t].mult = 0.0;
                } else {
                    ctx.jstate[t].mult += j.extra(1.0);
                }
            }
            "j_obelisk" if !copied => {
                let this = s.levels.get(det.name).map_or(0.0, |l| l.played) + 1.0;
                let outplayed =
                    s.levels.iter().any(|(k, l)| k != det.name && l.played >= this && l.visible);
                if outplayed {
                    ctx.jstate[t].x_mult += j.extra(0.2);
                } else {
                    ctx.jstate[t].x_mult = 1.0;
                }
            }
            "j_green_joker" if !copied => ctx.jstate[t].mult += j.extra_num("hand_add", 1.0),
            _ => {}
        }
    }
}

/// context.repetition for a scoring card.
pub fn card_repetitions(ctx: &ScoreCtx, i: usize) -> u32 {
    let s = ctx.s;
    let card = &ctx.played[i];
    let mut reps = 0.0;
    for (t, _) in resolved_jokers(s, ctx.order) {
        let j = &s.jokers[t];
        match j.key.as_deref().unwrap_or("") {
            "j_sock_and_buskin" if hand::is_face(card, &ctx.rules) => reps += j.extra(1.0),
            "j_hanging_chad" if ctx.det.scoring.first() == Some(&i) => reps += j.extra(2.0),
            "j_dusk" if ctx.hands_left == 0.0 => reps += j.extra(1.0),
            "j_selzer" => reps += 1.0,
            "j_hack" if is_rank(card, &[2, 3, 4, 5]) => reps += j.extra(1.0),
            _ => {}
        }
    }
    reps as u32
}

/// context.repetition for a held card.
pub fn held_repetitions(ctx: &ScoreCtx) -> u32 {
    let s = ctx.s;
    resolved_jokers(s, ctx.order)
        .iter()
        .filter(|(t, _)| s.jokers[*t].is("j_mime"))
        .map(|(t, _)| s.jokers[*t].extra(1.0) as u32)
        .sum()
}

fn is_rank(card: &Card, ranks: &[i32]) -> bool {
    hand::rank_of(card).is_some_and(|r| ranks.contains(&r))
}

fn first_face(ctx: &ScoreCtx) -> Option<usize> {
    ctx.det.scoring.iter().copied().find(|&i| hand::is_face(&ctx.played[i], &ctx.rules))
}

/// context.individual for a scoring card.
pub fn per_card(ctx: &mut ScoreCtx, i: usize, lucky_p: f64) {
    let s = ctx.s;
    for (t, copied) in resolved_jokers(s, ctx.order) {
        let j = &s.jokers[t];
        let rules = ctx.rules;
        let card = ctx.played[i].clone();
        let face = hand::is_face(&card, &rules);
        match j.key.as_deref().unwrap_or("") {
            "j_hiker" => ctx.perma_add[i] += j.extra(5.0),
            "j_lucky_cat" if !copied && lucky_p > 0.0 => {
                let w = ctx.weight(lucky_p);
                ctx.jstate[t].x_mult += j.extra(0.25) * w;
            }
            "j_wee" if !copied && is_rank(&card, &[2]) => {
                ctx.jstate[t].chips += j.extra_num("chip_mod", 8.0)
            }
            "j_photograph" if first_face(ctx) == Some(i) => ctx.x_mult(j.extra(2.0)),
            "j_idol"
                if hand::rank_of(&card) == Some(s.round.idol.rank)
                    && hand::is_suit(&card, &s.round.idol.suit, &rules) =>
            {
                ctx.x_mult(j.extra(2.0))
            }
            "j_scary_face" if face => ctx.chips += j.extra(30.0),
            "j_smiley" if face => ctx.mult += j.extra(5.0),
            "j_ticket" if card.has_enh("m_gold") => ctx.dollars += j.extra(4.0),
            "j_scholar" if is_rank(&card, &[14]) => {
                ctx.chips += j.extra_num("chips", 20.0);
                ctx.mult += j.extra_num("mult", 4.0);
            }
            "j_walkie_talkie" if is_rank(&card, &[10, 4]) => {
                ctx.chips += j.extra_num("chips", 10.0);
                ctx.mult += j.extra_num("mult", 4.0);
            }
            "j_business" if face => {
                let p = ctx.prob(j.extra(2.0));
                ctx.chance_dollars(p, 2.0);
            }
            "j_fibonacci" if is_rank(&card, &[2, 3, 5, 8, 14]) => ctx.mult += j.extra(8.0),
            "j_even_steven" if is_rank(&card, &[2, 4, 6, 8, 10]) => ctx.mult += j.extra(4.0),
            "j_odd_todd" if is_rank(&card, &[3, 5, 7, 9, 14]) => ctx.chips += j.extra(31.0),
            "j_greedy_joker" | "j_lusty_joker" | "j_wrathful_joker" | "j_gluttenous_joker"
                if hand::is_suit(&card, j.extra_str("suit").unwrap_or(""), &rules) =>
            {
                ctx.mult += j.extra_num("s_mult", 3.0)
            }
            "j_rough_gem" if hand::is_suit(&card, "Diamonds", &rules) => {
                ctx.dollars += j.extra(1.0)
            }
            "j_onyx_agate" if hand::is_suit(&card, "Clubs", &rules) => ctx.mult += j.extra(7.0),
            "j_arrowhead" if hand::is_suit(&card, "Spades", &rules) => ctx.chips += j.extra(50.0),
            "j_bloodstone" if hand::is_suit(&card, "Hearts", &rules) => {
                let p = ctx.prob(j.extra_num("odds", 2.0));
                ctx.chance_x_mult(p, j.extra_num("Xmult", 1.5));
            }
            "j_ancient" if hand::is_suit(&card, &s.round.ancient_suit, &rules) => {
                ctx.x_mult(j.extra(1.5))
            }
            "j_triboulet" if is_rank(&card, &[12, 13]) => ctx.x_mult(j.extra(2.0)),
            _ => {}
        }
    }
}

/// The held card Raised Fist picks: the lowest rank, rightmost on a tie.
fn raised_fist_card(ctx: &ScoreCtx) -> Option<usize> {
    let mut lowest: Option<(usize, i32)> = None;
    for (i, card) in ctx.held.iter().enumerate() {
        if let Some(rank) = hand::rank_of(card) {
            if lowest.is_none_or(|(_, r)| rank <= r) {
                lowest = Some((i, rank));
            }
        }
    }
    lowest.map(|(i, _)| i)
}

/// context.individual for a card held in hand. Debuffed cards never get here.
pub fn held_card(ctx: &mut ScoreCtx, i: usize) {
    let s = ctx.s;
    for (t, _copied) in resolved_jokers(s, ctx.order) {
        let j = &s.jokers[t];
        let card = ctx.held[i].clone();
        match j.key.as_deref().unwrap_or("") {
            "j_shoot_the_moon" if is_rank(&card, &[12]) => ctx.mult += 13.0,
            "j_baron" if is_rank(&card, &[13]) => ctx.x_mult(j.extra(1.5)),
            "j_reserved_parking" if hand::is_face(&card, &ctx.rules) => {
                let p = ctx.prob(j.extra_num("odds", 2.0));
                ctx.chance_dollars(p, j.extra_num("dollars", 1.0));
            }
            "j_raised_fist" if raised_fist_card(ctx) == Some(i) => {
                let nominal = match card.rank {
                    14 => 11.0,
                    11..=13 => 10.0,
                    r => r as f64,
                };
                ctx.mult += 2.0 * nominal;
            }
            _ => {}
        }
    }
}

/// Flower Pot's check, from its joker_main.
fn flower_pot(ctx: &ScoreCtx) -> bool {
    let suits = ["Hearts", "Diamonds", "Spades", "Clubs"];
    let mut seen = [false; 4];
    let cards: Vec<&Card> = ctx.det.scoring.iter().map(|&i| ctx.played[i].as_ref()).collect();
    for card in cards.iter().filter(|c| !c.has_enh("m_wild")) {
        if let Some(k) =
            (0..4).find(|&k| !seen[k] && hand::is_suit_bypass(card, suits[k], true, &ctx.rules))
        {
            seen[k] = true;
        }
    }
    for card in cards.iter().filter(|c| c.has_enh("m_wild")) {
        if let Some(k) = (0..4).find(|&k| !seen[k] && hand::is_suit(card, suits[k], &ctx.rules)) {
            seen[k] = true;
        }
    }
    seen.iter().all(|&x| x)
}

/// SMODS.seeing_double_check for Clubs.
fn seeing_double(ctx: &ScoreCtx) -> bool {
    let suits = ["Clubs", "Hearts", "Diamonds", "Spades"];
    let mut tally = [0u32; 4];
    let cards: Vec<&Card> = ctx.det.scoring.iter().map(|&i| ctx.played[i].as_ref()).collect();
    for card in cards.iter().filter(|c| !c.has_enh("m_wild")) {
        for k in 0..4 {
            if hand::is_suit(card, suits[k], &ctx.rules) {
                tally[k] += 1;
            }
        }
    }
    for card in cards.iter().filter(|c| c.has_enh("m_wild")) {
        if hand::is_suit(card, "Clubs", &ctx.rules) && tally[0] == 0 {
            tally[0] = 1;
        } else {
            for k in 0..4 {
                if hand::is_suit(card, suits[k], &ctx.rules) && tally[k] == 0 {
                    tally[k] = 1;
                }
            }
        }
    }
    tally[0] > 0 && tally[1..].iter().any(|&n| n > 0)
}

fn loyalty_value(ctx: &ScoreCtx, j: &Joker) -> f64 {
    let every = j.extra_num("every", 5.0);
    let played = ctx.s.round.hands_played - j.num("hands_played_at_create", 0.0);
    (every - 1.0 - played).rem_euclid(every + 1.0)
}

fn joker_main(ctx: &mut ScoreCtx, t: usize) {
    let s = ctx.s;
    let j = &s.jokers[t];
    let key = j.key.as_deref().unwrap_or("");
    if key == "j_loyalty_card" {
        // The counter is recomputed before the copy check, so copies see the fresh value.
        if loyalty_value(ctx, j) == j.extra_num("every", 5.0) {
            ctx.x_mult(j.extra_num("Xmult", 4.0));
        }
        return;
    }
    let state = ctx.jstate[t];
    let hand_type = j.ab.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if key != "j_seeing_double"
        && state.x_mult > 1.0
        && (hand_type.is_empty() || ctx.det.has(hand_type))
    {
        ctx.x_mult(state.x_mult);
        return;
    }
    let t_mult = j.num("t_mult", 0.0);
    if t_mult > 0.0 && ctx.det.has(hand_type) {
        ctx.mult += t_mult;
        return;
    }
    let t_chips = j.num("t_chips", 0.0);
    if t_chips > 0.0 && ctx.det.has(hand_type) {
        ctx.chips += t_chips;
        return;
    }
    let round = &s.round;
    match key {
        "j_half" if ctx.played.len() as f64 <= j.extra_num("size", 3.0) => {
            ctx.mult += j.extra_num("mult", 20.0)
        }
        "j_abstract" => ctx.mult += s.jokers.len() as f64 * j.extra(3.0),
        "j_acrobat" if ctx.hands_left == 0.0 => ctx.x_mult(j.extra(3.0)),
        "j_mystic_summit" if round.discards_left == j.extra_num("d_remaining", 0.0) => {
            ctx.mult += j.extra_num("mult", 15.0)
        }
        "j_misprint" => {
            let (min, max) = (j.extra_num("min", 0.0), j.extra_num("max", 23.0));
            ctx.mult += ctx.chance.pick(min, max);
        }
        "j_banner" if round.discards_left > 0.0 => ctx.chips += round.discards_left * j.extra(30.0),
        "j_stuntman" => ctx.chips += j.extra_num("chip_mod", 250.0),
        "j_matador" if ctx.blind_triggered => ctx.dollars += j.extra(8.0),
        "j_supernova" => {
            ctx.mult += s.levels.get(ctx.det.name).map_or(0.0, |l| l.played) + 1.0;
        }
        "j_flower_pot" if flower_pot(ctx) => ctx.x_mult(j.extra(3.0)),
        "j_seeing_double" if seeing_double(ctx) => ctx.x_mult(j.extra(2.0)),
        "j_castle" | "j_wee" | "j_square" | "j_runner" | "j_ice_cream" => ctx.chips += state.chips,
        "j_blue_joker" if s.deck.remaining > 0.0 => ctx.chips += j.extra(2.0) * s.deck.remaining,
        "j_erosion" if s.deck.starting - s.deck.total - ctx.cards_added > 0.0 => {
            ctx.mult += j.extra(4.0) * (s.deck.starting - s.deck.total - ctx.cards_added)
        }
        "j_stone" => ctx.chips += j.extra(25.0) * j.num("stone_tally", 0.0),
        "j_steel_joker" if j.num("steel_tally", 0.0) > 0.0 => {
            ctx.x_mult(1.0 + j.extra(0.2) * j.num("steel_tally", 0.0))
        }
        "j_bull" if ctx.dollars > 0.0 => ctx.chips += j.extra(2.0) * ctx.dollars,
        "j_drivers_license" if j.num("driver_tally", 0.0) >= 16.0 => ctx.x_mult(j.extra(3.0)),
        "j_blackboard"
            if ctx.held.iter().all(|c| {
                hand::flush_matches(c, "Clubs", &ctx.rules)
                    || hand::flush_matches(c, "Spades", &ctx.rules)
            }) =>
        {
            ctx.x_mult(j.extra(3.0))
        }
        "j_ceremonial" | "j_swashbuckler" | "j_joker" | "j_trousers" | "j_ride_the_bus"
        | "j_flash" | "j_popcorn" | "j_green_joker" | "j_red_card" => {
            if state.mult > 0.0 {
                ctx.mult += state.mult;
            }
        }
        "j_fortune_teller" if round.tarot_used > 0.0 => ctx.mult += round.tarot_used,
        "j_gros_michel" => ctx.mult += j.extra_num("mult", 15.0),
        "j_cavendish" => ctx.x_mult(j.extra_num("Xmult", 3.0)),
        "j_card_sharp" if s.levels.get(ctx.det.name).is_some_and(|l| l.played_round > 0.0) => {
            ctx.x_mult(j.extra_num("Xmult", 3.0))
        }
        "j_bootstraps" => {
            let steps = (ctx.dollars / j.extra_num("dollars", 5.0)).floor();
            if steps >= 1.0 {
                ctx.mult += j.extra_num("mult", 2.0) * steps;
            }
        }
        "j_caino" if j.num("caino_xmult", 1.0) > 1.0 => ctx.x_mult(j.num("caino_xmult", 1.0)),
        _ => {}
    }
}

fn apply_joker_edition(ctx: &mut ScoreCtx, ed: Option<&str>, pre: bool) {
    match (ed, pre) {
        (Some("e_foil"), true) => ctx.chips += 50.0,
        (Some("e_holo"), true) => ctx.mult += 10.0,
        (Some("e_polychrome"), false) => ctx.mult *= 1.5,
        _ => {}
    }
}

/// The joker loop of evaluate_play: edition, joker_main, other_joker, edition.
pub fn main_phase(ctx: &mut ScoreCtx) {
    let s = ctx.s;
    let order = ctx.order;
    for pos in 0..order.len() {
        let index = order[pos];
        let joker = &s.jokers[index];
        if !joker.debuff {
            apply_joker_edition(ctx, joker.ed.as_deref(), true);
        }
        if let Some((t, _)) = resolve(s, order, pos) {
            joker_main(ctx, t);
        }
        if joker.rarity == 2 {
            for (other, _) in resolved_jokers(s, order) {
                if s.jokers[other].is("j_baseball") && other != index {
                    ctx.x_mult(s.jokers[other].extra(1.5));
                }
            }
        }
        if !joker.debuff {
            apply_joker_edition(ctx, joker.ed.as_deref(), false);
        }
    }
}
