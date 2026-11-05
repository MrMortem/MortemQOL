//! Boss blind effects on a play, following Blind:debuff_hand and Blind:modify_hand.
use crate::snapshot::Snapshot;

fn active(s: &Snapshot, key: &str) -> bool {
    !s.blind.disabled && s.blind.key == key
}

/// True when the blind makes this hand score nothing.
pub fn debuffs_hand(s: &Snapshot, hand: &str, played_count: usize) -> bool {
    if active(s, "bl_psychic") && played_count < 5 {
        return true;
    }
    let played_round = |name: &str| s.levels.get(name).is_some_and(|l| l.played_round > 0.0);
    if active(s, "bl_eye") && played_round(hand) {
        return true;
    }
    if active(s, "bl_mouth") && s.levels.iter().any(|(k, l)| k != hand && l.played_round > 0.0) {
        return true;
    }
    false
}

/// The Arm lowers the played hand's level before scoring.
pub fn arm_lowers(s: &Snapshot, level: f64) -> bool {
    active(s, "bl_arm") && level > 1.0
}

/// The Ox takes all money when the most played hand is played.
pub fn ox_takes_money(s: &Snapshot, hand: &str) -> bool {
    active(s, "bl_ox") && s.round.most_played == hand
}

pub fn flint_active(s: &Snapshot) -> bool {
    active(s, "bl_flint")
}
