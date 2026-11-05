mod common;
use common::{card, cards};
use mortemqol_engine::hand::{detect, is_face, Rules};
use mortemqol_engine::snapshot::Card;

fn run(codes: &str, rules: &Rules) -> (String, Vec<usize>) {
    let cs = cards(codes);
    let refs: Vec<&Card> = cs.iter().collect();
    let d = detect(&refs, rules);
    (d.name.to_string(), d.scoring)
}

fn name(codes: &str) -> String {
    run(codes, &Rules::default()).0
}

#[test]
fn every_hand_type_detected() {
    assert_eq!(name("AS"), "High Card");
    assert_eq!(name("AS AH"), "Pair");
    assert_eq!(name("AS AH KS KH"), "Two Pair");
    assert_eq!(name("AS AH AD"), "Three of a Kind");
    assert_eq!(name("2S 3H 4D 5C 6S"), "Straight");
    assert_eq!(name("2S 9S 4S 5S KS"), "Flush");
    assert_eq!(name("AS AH AD KS KH"), "Full House");
    assert_eq!(name("AS AH AD AC"), "Four of a Kind");
    assert_eq!(name("2S 3S 4S 5S 6S"), "Straight Flush");
    assert_eq!(name("10S JS QS KS AS"), "Straight Flush"); // the game's key for a Royal Flush
    assert_eq!(name("AS AS AH AD AC"), "Five of a Kind");
    assert_eq!(name("AS AS AS KS KS"), "Flush House");
    assert_eq!(name("AS AS AS AS AS"), "Flush Five");
}

#[test]
fn kicker_excluded_from_scoring() {
    assert_eq!(run("AS AH KD", &Rules::default()), ("Pair".into(), vec![0, 1]));
}

#[test]
fn high_card_picks_highest_rank_only() {
    assert_eq!(run("3S KD 9H", &Rules::default()), ("High Card".into(), vec![1]));
}

#[test]
fn two_pair_excludes_kicker_in_play_order() {
    assert_eq!(run("KS 2D AS KH AH", &Rules::default()), ("Two Pair".into(), vec![0, 2, 3, 4]));
}

#[test]
fn ace_is_low_or_high_but_never_wraps() {
    assert_eq!(name("AS 2H 3D 4C 5S"), "Straight");
    assert_eq!(name("10S JH QD KC AS"), "Straight");
    assert_eq!(name("QS KH AD 2C 3S"), "High Card");
}

#[test]
fn four_fingers_flush() {
    let rules = Rules { four_fingers: true, ..Rules::default() };
    assert_eq!(run("2S 9S 4S KS 7H", &rules), ("Flush".into(), vec![0, 1, 2, 3]));
    assert_eq!(name("2S 9S 4S KS 7H"), "High Card");
}

#[test]
fn four_fingers_straight_includes_duplicate_rank() {
    let rules = Rules { four_fingers: true, ..Rules::default() };
    assert_eq!(run("2S 3H 4D 5C 5S", &rules), ("Straight".into(), vec![0, 1, 2, 3, 4]));
}

#[test]
fn shortcut_straight() {
    let rules = Rules { shortcut: true, ..Rules::default() };
    assert_eq!(run("2S 4H 6D 8C 10S", &rules).0, "Straight");
    assert_eq!(name("2S 4H 6D 8C 10S"), "High Card");
}

#[test]
fn smeared_flush_mixes_red() {
    let rules = Rules { smeared: true, ..Rules::default() };
    assert_eq!(run("2H 9D 4H KD 7H", &rules).0, "Flush");
    assert_eq!(name("2H 9D 4H KD 7H"), "High Card");
}

#[test]
fn wild_card_counts_for_any_suit() {
    let mut cs = cards("2S 9S 4S KS 7H");
    cs[4].enh = Some("m_wild".into());
    let refs: Vec<&Card> = cs.iter().collect();
    assert_eq!(detect(&refs, &Rules::default()).name, "Flush");
}

#[test]
fn debuffed_wild_card_keeps_only_its_own_suit() {
    let mut cs = cards("2S 9S 4S KS 7H");
    cs[4].enh = Some("m_wild".into());
    cs[4].debuff = true;
    let refs: Vec<&Card> = cs.iter().collect();
    assert_eq!(detect(&refs, &Rules::default()).name, "High Card");
}

#[test]
fn stone_card_has_no_rank_and_always_scores() {
    let mut cs = cards("AS AH AD");
    cs[2].enh = Some("m_stone".into());
    let refs: Vec<&Card> = cs.iter().collect();
    let d = detect(&refs, &Rules::default());
    assert_eq!(d.name, "Pair");
    assert_eq!(d.scoring, vec![0, 1, 2]);
}

#[test]
fn splash_scores_all_played() {
    let rules = Rules { splash: true, ..Rules::default() };
    assert_eq!(run("AS AH KD", &rules), ("Pair".into(), vec![0, 1, 2]));
}

#[test]
fn pareidolia_makes_all_cards_face() {
    let two = card("2S");
    assert!(!is_face(&two, &Rules::default()));
    assert!(is_face(&two, &Rules { pareidolia: true, ..Rules::default() }));
    assert!(is_face(&card("KS"), &Rules::default()));
    let mut debuffed = card("KS");
    debuffed.debuff = true;
    assert!(!is_face(&debuffed, &Rules::default()));
}
