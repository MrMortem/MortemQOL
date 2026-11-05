//! Scores one play. This follows G.FUNCS.evaluate_play and SMODS.score_card without touching
//! any game state.
use crate::blinds;
use crate::hand::{self, Detected, Rules};
use crate::jokers;
use crate::snapshot::{self, Card, Snapshot};
use serde::Deserialize;
use serde_json::{json, Value};
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    /// No chance effect fires.
    Floor,
    /// Each chance effect is replaced by its mean.
    Expected,
    /// Every chance effect fires at its highest.
    Ceiling,
}

/// Joker values that can change during a hand.
#[derive(Debug, Clone, Copy, Default)]
pub struct JState {
    pub mult: f64,
    pub x_mult: f64,
    pub chips: f64,
    /// Set once Loyalty Card has updated its counter this hand.
    pub loyalty: Option<f64>,
}

pub struct ScoreCtx<'a> {
    pub s: &'a Snapshot,
    pub mode: Mode,
    pub order: &'a [usize],
    pub played: Vec<Cow<'a, Card>>,
    pub held: Vec<Cow<'a, Card>>,
    pub det: &'a Detected,
    pub rules: Rules,
    pub chips: f64,
    pub mult: f64,
    pub level_chips: f64,
    pub level_mult: f64,
    pub dollars: f64,
    pub hands_left: f64,
    pub blind_triggered: bool,
    pub chance_used: bool,
    pub oops: f64,
    pub jstate: Vec<JState>,
    pub perma_add: Vec<f64>,
}

impl ScoreCtx<'_> {
    /// Probability of a `1 in denominator` effect, after Oops! All 6s.
    pub fn prob(&self, denominator: f64) -> f64 {
        if denominator <= 0.0 {
            return 1.0;
        }
        (self.oops / denominator).min(1.0)
    }

    /// Weight of an effect that fires with probability `p` in the current mode.
    pub fn weight(&mut self, p: f64) -> f64 {
        if p >= 1.0 {
            return 1.0;
        }
        if p <= 0.0 {
            return 0.0;
        }
        self.chance_used = true;
        match self.mode {
            Mode::Floor => 0.0,
            Mode::Expected => p,
            Mode::Ceiling => 1.0,
        }
    }

    pub fn x_mult(&mut self, x: f64) {
        self.mult *= x;
    }

    pub fn chance_mult(&mut self, p: f64, amount: f64) {
        self.mult += amount * self.weight(p);
    }

    pub fn chance_x_mult(&mut self, p: f64, x: f64) {
        let w = self.weight(p);
        self.mult *= 1.0 + (x - 1.0) * w;
    }

    pub fn chance_dollars(&mut self, p: f64, amount: f64) {
        self.dollars += amount * self.weight(p);
    }
}

#[derive(Debug, Clone)]
pub struct Scored {
    pub hand: String,
    /// Card ids that score.
    pub scored: Vec<i64>,
    pub floor: f64,
    pub expected: f64,
    pub ceiling: f64,
    pub legal: bool,
    pub has_chance: bool,
    pub approximate: Vec<String>,
}

/// Numbers that JSON cannot represent cross the boundary as "naneinf".
pub fn to_json_number(x: f64) -> Value {
    if x.is_finite() {
        json!(x)
    } else {
        json!("naneinf")
    }
}

impl Scored {
    pub fn to_json(&self) -> Value {
        json!({
            "hand": self.hand,
            "scored": self.scored,
            "floor": to_json_number(self.floor),
            "expected": to_json_number(self.expected),
            "ceiling": to_json_number(self.ceiling),
            "legal": self.legal,
            "has_chance": self.has_chance,
            "approximate": self.approximate,
        })
    }
}

const KNOWN_ENHANCEMENTS: [&str; 8] =
    ["m_bonus", "m_mult", "m_wild", "m_glass", "m_steel", "m_stone", "m_gold", "m_lucky"];
const KNOWN_EDITIONS: [&str; 4] = ["e_foil", "e_holo", "e_polychrome", "e_negative"];
const KNOWN_SEALS: [&str; 4] = ["Red", "Blue", "Gold", "Purple"];

fn push_unique(tags: &mut Vec<String>, tag: String) {
    if !tags.contains(&tag) {
        tags.push(tag);
    }
}

/// The reasons a snapshot's results cannot be exact.
pub fn approximate_tags(s: &Snapshot) -> Vec<String> {
    let mut tags = Vec::new();
    for card in &s.hand {
        if card.hidden {
            push_unique(&mut tags, "hidden_card".to_string());
        }
        if let Some(e) = card.enh.as_deref().filter(|e| !KNOWN_ENHANCEMENTS.contains(e)) {
            push_unique(&mut tags, format!("unknown_enhancement:{e}"));
        }
        if let Some(e) = card.ed.as_deref().filter(|e| !KNOWN_EDITIONS.contains(e)) {
            push_unique(&mut tags, format!("unknown_edition:{e}"));
        }
        if let Some(e) = card.seal.as_deref().filter(|e| !KNOWN_SEALS.contains(e)) {
            push_unique(&mut tags, format!("unknown_seal:{e}"));
        }
    }
    for joker in &s.jokers {
        if joker.hidden {
            push_unique(&mut tags, "hidden_joker".to_string());
        } else if let Some(key) = joker.key.as_deref().filter(|k| !jokers::is_known(k)) {
            push_unique(&mut tags, format!("unknown_joker:{key}"));
        }
    }
    if !s.blind.disabled && s.blind.key == "bl_hook" {
        push_unique(&mut tags, "hook".to_string());
    }
    if s.lobby {
        push_unique(&mut tags, "lobby".to_string());
    }
    tags
}

fn chip_value(card: &Card) -> f64 {
    match hand::rank_of(card) {
        Some(14) => 11.0,
        Some(11..=13) => 10.0,
        Some(r) => r as f64,
        None => 0.0,
    }
}

/// Base chips a played card gives, from Card:get_chip_bonus.
fn card_chips(card: &Card) -> f64 {
    let bonus = card.perma.bonus;
    match card.enh.as_deref() {
        Some("m_stone") => 50.0 + bonus,
        Some("m_bonus") => chip_value(card) + 30.0 + bonus,
        _ => chip_value(card) + bonus,
    }
}

fn apply_edition(ctx: &mut ScoreCtx, ed: Option<&str>) {
    match ed {
        Some("e_foil") => ctx.chips += 50.0,
        Some("e_holo") => ctx.mult += 10.0,
        Some("e_polychrome") => ctx.mult *= 1.5,
        _ => {}
    }
}

fn score_played_card(ctx: &mut ScoreCtx, i: usize) {
    let card = ctx.played[i].clone();
    if card.debuff {
        ctx.blind_triggered = true;
        return;
    }
    let reps = 1 + u32::from(card.is_seal("Red")) + jokers::card_repetitions(ctx, i);
    for _ in 0..reps {
        ctx.chips += card_chips(&card) + ctx.perma_add[i];
        ctx.mult += card.perma.mult;
        let mut lucky_p = 0.0;
        match card.enh.as_deref() {
            Some("m_mult") => ctx.mult += 4.0,
            Some("m_lucky") => {
                let (p_mult, p_money) = (ctx.prob(5.0), ctx.prob(15.0));
                ctx.chance_mult(p_mult, 20.0);
                lucky_p = 1.0 - (1.0 - p_mult) * (1.0 - p_money);
                ctx.chance_dollars(p_money, 20.0);
            }
            Some("m_glass") => ctx.x_mult(2.0),
            _ => {}
        }
        if card.perma.x_mult != 1.0 && card.perma.x_mult > 0.0 {
            ctx.x_mult(card.perma.x_mult);
        }
        if card.is_seal("Gold") {
            ctx.dollars += 3.0;
        }
        apply_edition(ctx, card.ed.as_deref());
        jokers::per_card(ctx, i, lucky_p);
    }
}

fn score_held_card(ctx: &mut ScoreCtx, i: usize) {
    if ctx.held[i].debuff || ctx.held[i].hidden {
        return;
    }
    let steel = ctx.held[i].has_enh("m_steel");
    let reps = 1 + u32::from(ctx.held[i].is_seal("Red")) + jokers::held_repetitions(ctx);
    for _ in 0..reps {
        if steel {
            ctx.x_mult(1.5);
        }
        jokers::held_card(ctx, i);
    }
}

fn observatory(ctx: &mut ScoreCtx) {
    if !ctx.s.vouchers.iter().any(|v| v == "v_observatory") {
        return;
    }
    let count =
        ctx.s.consumables.iter().filter(|c| planet_hand(&c.key) == Some(ctx.det.name)).count();
    for _ in 0..count {
        ctx.x_mult(1.5);
    }
}

fn planet_hand(key: &str) -> Option<&'static str> {
    Some(match key {
        "c_pluto" => "High Card",
        "c_mercury" => "Pair",
        "c_uranus" => "Two Pair",
        "c_venus" => "Three of a Kind",
        "c_saturn" => "Straight",
        "c_jupiter" => "Flush",
        "c_earth" => "Full House",
        "c_mars" => "Four of a Kind",
        "c_neptune" => "Straight Flush",
        "c_planet_x" => "Five of a Kind",
        "c_ceres" => "Flush House",
        "c_eris" => "Flush Five",
        _ => return None,
    })
}

/// Runs one evaluation. `level_ups` is how many Space Joker triggers fire.
fn run(
    s: &Snapshot,
    play: &[usize],
    order: &[usize],
    det: &Detected,
    rules: Rules,
    mode: Mode,
    level_ups: u32,
) -> (f64, bool) {
    let played: Vec<Cow<Card>> = play.iter().map(|&i| Cow::Borrowed(&s.hand[i])).collect();
    let held: Vec<Cow<Card>> = s
        .hand
        .iter()
        .enumerate()
        .filter(|(i, _)| !play.contains(i))
        .map(|(_, c)| Cow::Borrowed(c))
        .collect();
    let level = s.levels.get(det.name);
    let (mut lvl_chips, mut lvl_mult) = level.map_or((0.0, 1.0), |l| (l.chips, l.mult));
    let (l_chips, l_mult) = level.map_or((0.0, 0.0), |l| (l.l_chips, l.l_mult));
    let mut blind_triggered = false;
    if level.is_some_and(|l| blinds::arm_lowers(s, l.level)) {
        lvl_chips = (lvl_chips - l_chips).max(0.0);
        lvl_mult = (lvl_mult - l_mult).max(1.0);
        blind_triggered = true;
    }
    let mut dollars = s.round.dollars;
    if blinds::ox_takes_money(s, det.name) {
        dollars = 0.0;
        blind_triggered = true;
    }
    let oops = s.jokers.iter().filter(|j| j.is("j_oops") && !j.debuff).count();
    let mut ctx = ScoreCtx {
        s,
        mode,
        order,
        perma_add: vec![0.0; played.len()],
        played,
        held,
        det,
        rules,
        chips: 0.0,
        mult: 0.0,
        level_chips: lvl_chips + l_chips * level_ups as f64,
        level_mult: lvl_mult + l_mult * level_ups as f64,
        dollars,
        hands_left: s.round.hands_left - 1.0,
        blind_triggered,
        chance_used: false,
        oops: 2f64.powi(oops as i32),
        jstate: jokers::initial_state(s),
    };

    jokers::before(&mut ctx);
    ctx.chips = ctx.level_chips;
    ctx.mult = ctx.level_mult;
    if blinds::flint_active(s) {
        ctx.mult = (ctx.mult * 0.5 + 0.5).floor().max(1.0);
        ctx.chips = (ctx.chips * 0.5 + 0.5).floor().max(0.0);
        ctx.blind_triggered = true;
    }
    for i in 0..ctx.played.len() {
        if det.scoring.contains(&i) {
            score_played_card(&mut ctx, i);
        }
    }
    for i in 0..ctx.held.len() {
        score_held_card(&mut ctx, i);
    }
    jokers::main_phase(&mut ctx);
    observatory(&mut ctx);
    if s.deck.back == "b_plasma" {
        let half = ((ctx.chips + ctx.mult) / 2.0).floor();
        ctx.chips = half;
        ctx.mult = half;
    }
    (ctx.chips * ctx.mult, ctx.chance_used)
}

fn binomial(n: u32, k: u32) -> f64 {
    (0..k).fold(1.0, |acc, i| acc * (n - i) as f64 / (i + 1) as f64)
}

/// Scores `play` (indexes into `s.hand`, in play order) with jokers in `order` (indexes into
/// `s.jokers`).
pub fn score_play(s: &Snapshot, play: &[usize], order: &[usize]) -> Scored {
    let rules = jokers::rules(s);
    let refs: Vec<&Card> = play.iter().map(|&i| &s.hand[i]).collect();
    let det = hand::detect(&refs, &rules);
    let scored = det.scoring.iter().map(|&i| refs[i].id).collect();
    let mut out = Scored {
        hand: det.name.to_string(),
        scored,
        floor: 0.0,
        expected: 0.0,
        ceiling: 0.0,
        legal: true,
        has_chance: false,
        approximate: approximate_tags(s),
    };
    if blinds::debuffs_hand(s, det.name, play.len()) {
        out.legal = false;
        return out;
    }
    let triggers = jokers::space_triggers(s, order);
    let oops = s.jokers.iter().filter(|j| j.is("j_oops") && !j.debuff).count();
    let p = (2f64.powi(oops as i32) / jokers::space_odds(s)).min(1.0);
    let run_at = |mode, k| run(s, play, order, &det, rules, mode, k);
    if triggers == 0 || p >= 1.0 {
        let k = if p >= 1.0 { triggers } else { 0 };
        let (expected, chance) = run_at(Mode::Expected, k);
        out.expected = expected;
        out.has_chance = chance;
        if chance {
            out.floor = run_at(Mode::Floor, k).0;
            out.ceiling = run_at(Mode::Ceiling, k).0;
        } else {
            out.floor = expected;
            out.ceiling = expected;
        }
        return out;
    }
    out.has_chance = true;
    out.floor = run_at(Mode::Floor, 0).0;
    out.ceiling = run_at(Mode::Ceiling, triggers).0;
    out.expected = (0..=triggers)
        .map(|k| {
            let w =
                binomial(triggers, k) * p.powi(k as i32) * (1.0 - p).powi((triggers - k) as i32);
            w * run_at(Mode::Expected, k).0
        })
        .sum();
    out
}

/// Just the expected score, for the search's hot loop.
pub fn score_expected(s: &Snapshot, play: &[usize], order: &[usize]) -> f64 {
    let rules = jokers::rules(s);
    let refs: Vec<&Card> = play.iter().map(|&i| &s.hand[i]).collect();
    let det = hand::detect(&refs, &rules);
    if blinds::debuffs_hand(s, det.name, play.len()) {
        return 0.0;
    }
    let triggers = jokers::space_triggers(s, order);
    if triggers == 0 {
        return run(s, play, order, &det, rules, Mode::Expected, 0).0;
    }
    let oops = s.jokers.iter().filter(|j| j.is("j_oops") && !j.debuff).count();
    let p = (2f64.powi(oops as i32) / jokers::space_odds(s)).min(1.0);
    if p >= 1.0 {
        return run(s, play, order, &det, rules, Mode::Expected, triggers).0;
    }
    (0..=triggers)
        .map(|k| {
            let w =
                binomial(triggers, k) * p.powi(k as i32) * (1.0 - p).powi((triggers - k) as i32);
            w * run(s, play, order, &det, rules, Mode::Expected, k).0
        })
        .sum()
}

#[derive(Deserialize)]
struct ScoreRequest {
    snapshot: Value,
    play: Vec<i64>,
}

/// Handles an `mqol_score` call. Card ids in `play` are looked up in the snapshot's hand.
pub fn score_request(json: &[u8]) -> Result<Vec<u8>, String> {
    let req: ScoreRequest = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    let s = snapshot::parse(req.snapshot.to_string().as_bytes())?;
    let mut play = Vec::with_capacity(req.play.len());
    for id in req.play {
        let index = s.hand.iter().position(|c| c.id == id);
        play.push(index.ok_or_else(|| format!("card {id} is not in hand"))?);
    }
    let order: Vec<usize> = (0..s.jokers.len()).collect();
    serde_json::to_vec(&score_play(&s, &play, &order).to_json()).map_err(|e| e.to_string())
}
