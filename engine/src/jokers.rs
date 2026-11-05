//! Vanilla joker effects, following Card:calculate_joker. Values come from the joker's
//! `ability` table in the snapshot; the defaults are the game's own.
use crate::hand::Rules;
use crate::score::ScoreCtx;
use crate::snapshot::Snapshot;

pub fn is_known(_key: &str) -> bool {
    false
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

pub fn before(_ctx: &mut ScoreCtx) {}
pub fn card_repetitions(_ctx: &ScoreCtx, _card: usize) -> u32 {
    0
}
pub fn held_repetitions(_ctx: &ScoreCtx) -> u32 {
    0
}
pub fn per_card(_ctx: &mut ScoreCtx, _card: usize, _lucky_p: f64) {}
pub fn held_card(_ctx: &mut ScoreCtx, _card: usize) {}
pub fn main_phase(_ctx: &mut ScoreCtx) {}
pub fn space_triggers(_s: &Snapshot, _order: &[usize]) -> u32 {
    0
}

pub fn initial_state(s: &Snapshot) -> Vec<crate::score::JState> {
    vec![crate::score::JState::default(); s.jokers.len()]
}

/// Space Joker's `1 in N`, with the default from the game.
pub fn space_odds(s: &Snapshot) -> f64 {
    s.jokers.iter().find(|j| j.is("j_space")).map_or(4.0, |j| j.extra(4.0))
}
