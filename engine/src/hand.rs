//! Poker hand detection, following the game's get_flush, get_straight and get_X_same,
//! and Steamodded's hand definitions.
use crate::snapshot::Card;

#[derive(Debug, Clone, Copy, Default)]
pub struct Rules {
    pub four_fingers: bool,
    pub shortcut: bool,
    pub smeared: bool,
    pub pareidolia: bool,
    pub splash: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Detected {
    pub name: &'static str,
    /// Indexes into the played cards, in play order.
    pub scoring: Vec<usize>,
    /// The poker hands this play contains, as checked by jokers such as Jolly Joker.
    pub contains: Vec<&'static str>,
}

impl Detected {
    pub fn has(&self, name: &str) -> bool {
        self.contains.contains(&name)
    }
}

const SUITS: [&str; 4] = ["Spades", "Hearts", "Clubs", "Diamonds"];

fn is_red(suit: &str) -> bool {
    suit == "Hearts" || suit == "Diamonds"
}

/// Rank for matching purposes. Stone cards have none.
pub fn rank_of(card: &Card) -> Option<i32> {
    if card.is_stone() || card.hidden || card.rank <= 0 {
        None
    } else {
        Some(card.rank)
    }
}

pub fn is_face(card: &Card, rules: &Rules) -> bool {
    if card.debuff {
        return false;
    }
    rules.pareidolia || matches!(rank_of(card), Some(11..=13))
}

/// Card:is_suit outside of flush checks: debuffed cards match nothing.
pub fn is_suit(card: &Card, suit: &str, rules: &Rules) -> bool {
    is_suit_bypass(card, suit, false, rules)
}

/// Card:is_suit with its bypass_debuff argument.
pub fn is_suit_bypass(card: &Card, suit: &str, bypass_debuff: bool, rules: &Rules) -> bool {
    if (card.debuff && !bypass_debuff) || card.is_stone() {
        return false;
    }
    if card.has_enh("m_wild") {
        return true;
    }
    if rules.smeared && is_red(&card.suit) == is_red(suit) {
        return true;
    }
    card.suit == suit
}

pub fn flush_matches(card: &Card, suit: &str, rules: &Rules) -> bool {
    if card.is_stone() {
        return false;
    }
    if card.has_enh("m_wild") && !card.debuff {
        return true;
    }
    if rules.smeared && is_red(&card.suit) == is_red(suit) {
        return true;
    }
    card.suit == suit
}

fn get_flush(played: &[&Card], rules: &Rules) -> Option<Vec<usize>> {
    let need = if rules.four_fingers { 4 } else { 5 };
    if played.len() < need {
        return None;
    }
    for suit in SUITS {
        let found: Vec<usize> =
            (0..played.len()).filter(|&i| flush_matches(played[i], suit, rules)).collect();
        if found.len() >= need {
            return Some(found);
        }
    }
    None
}

fn next_rank(rank: i32) -> i32 {
    if rank == 14 {
        2
    } else {
        rank + 1
    }
}

fn next_ranks(rank: i32, start: bool, skip: bool) -> Vec<i32> {
    // The Ace is a straight's edge: it can start one, but nothing continues past it.
    if !start && rank == 14 {
        return Vec::new();
    }
    let step = next_rank(rank);
    let mut out = vec![step];
    if skip && step != 14 {
        out.push(next_rank(step));
    }
    out
}

fn get_straight(played: &[&Card], rules: &Rules) -> Option<Vec<usize>> {
    let need = if rules.four_fingers { 4 } else { 5 };
    if played.len() < need {
        return None;
    }
    let ranks: Vec<Option<i32>> = played.iter().map(|c| rank_of(c)).collect();
    let present = |r: i32| ranks.contains(&Some(r));

    let mut best: Option<Vec<i32>> = None;
    let mut tuples: Vec<Vec<i32>> = (2..=14).filter(|&r| present(r)).map(|r| vec![r]).collect();
    for len in 2..=played.len() + 1 {
        let mut grown = Vec::new();
        for tuple in &tuples {
            let mut extended = false;
            if len != played.len() + 1 {
                let last = tuple[len - 2];
                for next in next_ranks(last, len == 2, rules.shortcut) {
                    if present(next) {
                        let mut t = tuple.clone();
                        t.push(next);
                        grown.push(t);
                        extended = true;
                    }
                }
            }
            if len > need && !extended && best.as_ref().is_none_or(|b| b.len() < tuple.len()) {
                best = Some(tuple.clone());
            }
        }
        tuples = grown;
    }
    let best = best?;
    Some((0..played.len()).filter(|&i| ranks[i].is_some_and(|r| best.contains(&r))).collect())
}

/// Groups of at least `num` same-ranked cards, highest rank first.
fn get_x_same(num: usize, played: &[&Card]) -> Vec<Vec<usize>> {
    let mut out = Vec::new();
    for rank in (2..=14).rev() {
        let group: Vec<usize> =
            (0..played.len()).filter(|&i| rank_of(played[i]) == Some(rank)).collect();
        if group.len() >= num {
            out.push(group);
        }
    }
    out
}

/// Card:get_nominal, used to choose the High Card.
fn nominal(card: &Card) -> f64 {
    let Some(rank) = rank_of(card) else {
        return -10000.0;
    };
    let chips = match rank {
        14 => 11.0,
        11..=13 => 10.0,
        r => r as f64,
    };
    let face = match rank {
        11 => 0.1,
        12 => 0.2,
        13 => 0.3,
        14 => 0.4,
        _ => 0.0,
    };
    let suit = match card.suit.as_str() {
        "Diamonds" => 0.01,
        "Clubs" => 0.02,
        "Hearts" => 0.03,
        "Spades" => 0.04,
        _ => 0.0,
    };
    10.0 * chips + suit + 10.0 * face
}

fn merged(a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut out: Vec<usize> = a.iter().chain(b).copied().collect();
    out.sort_unstable();
    out.dedup();
    out
}

fn flat(groups: &[Vec<usize>]) -> Vec<usize> {
    let mut out: Vec<usize> = groups.iter().flatten().copied().collect();
    out.sort_unstable();
    out
}

type Classified = (&'static str, Vec<usize>, Vec<&'static str>);

fn classify(played: &[&Card], rules: &Rules) -> Classified {
    let flush = get_flush(played, rules);
    let straight = get_straight(played, rules);
    let twos = get_x_same(2, played);
    let threes = get_x_same(3, played);
    let fours = get_x_same(4, played);
    let fives = get_x_same(5, played);

    let mut contains: Vec<&'static str> = Vec::new();
    if !fives.is_empty() && flush.is_some() {
        contains.push("Flush Five");
    }
    if !threes.is_empty() && twos.len() >= 2 && flush.is_some() {
        contains.push("Flush House");
    }
    if !fives.is_empty() {
        contains.push("Five of a Kind");
    }
    if straight.is_some() && flush.is_some() {
        contains.push("Straight Flush");
    }
    if !fours.is_empty() {
        contains.push("Four of a Kind");
    }
    if !threes.is_empty() && twos.len() >= 2 {
        contains.push("Full House");
    }
    if flush.is_some() {
        contains.push("Flush");
    }
    if straight.is_some() {
        contains.push("Straight");
    }
    if !threes.is_empty() {
        contains.push("Three of a Kind");
    }
    if twos.len() >= 2 {
        contains.push("Two Pair");
    }
    if !twos.is_empty() {
        contains.push("Pair");
    }
    if !played.is_empty() {
        contains.push("High Card");
    }

    if let (Some(five), Some(f)) = (fives.first(), &flush) {
        return ("Flush Five", merged(five, f), contains);
    }
    if let (true, true, Some(f)) = (!threes.is_empty(), twos.len() >= 2, &flush) {
        return ("Flush House", merged(&flat(&twos), f), contains);
    }
    if let Some(five) = fives.first() {
        return ("Five of a Kind", five.clone(), contains);
    }
    if let (Some(s), Some(f)) = (&straight, &flush) {
        return ("Straight Flush", merged(s, f), contains);
    }
    if let Some(four) = fours.first() {
        return ("Four of a Kind", four.clone(), contains);
    }
    if !threes.is_empty() && twos.len() >= 2 {
        return ("Full House", flat(&twos), contains);
    }
    if let Some(f) = flush {
        return ("Flush", f, contains);
    }
    if let Some(s) = straight {
        return ("Straight", s, contains);
    }
    if let Some(three) = threes.first() {
        return ("Three of a Kind", three.clone(), contains);
    }
    if twos.len() >= 2 {
        return ("Two Pair", flat(&twos), contains);
    }
    if let Some(two) = twos.first() {
        return ("Pair", two.clone(), contains);
    }
    let mut highest: Option<usize> = None;
    for i in 0..played.len() {
        if highest.is_none_or(|h| nominal(played[i]) > nominal(played[h])) {
            highest = Some(i);
        }
    }
    ("High Card", highest.into_iter().collect(), contains)
}

pub fn detect(played: &[&Card], rules: &Rules) -> Detected {
    let (name, base, contains) = classify(played, rules);
    let scoring = (0..played.len())
        .filter(|&i| rules.splash || played[i].is_stone() || base.contains(&i))
        .collect();
    Detected { name, scoring, contains }
}
