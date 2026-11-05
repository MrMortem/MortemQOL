//! Finds the best play: all plays first, then joker orders, then card orders.
use crate::score::{approximate_tags, score_expected, score_play, to_json_number};
use crate::snapshot::Snapshot;
use serde_json::{json, Value};

const MAX_ORDER_EVALS: usize = 200_000;
const MAX_EXACT_JOKERS: usize = 6;
const TOP_PLAYS: usize = 32;
const EPS: f64 = 1e-9;

#[derive(Debug, Clone)]
pub struct Best {
    /// Card ids, left to right.
    pub play: Vec<i64>,
    pub hand: String,
    pub scored: Vec<i64>,
    pub floor: f64,
    pub expected: f64,
    pub ceiling: f64,
    pub blind_pct: f64,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub stage: u32,
    pub done: bool,
    pub best: Option<Best>,
    /// Joker ids, left to right.
    pub joker_order: Vec<i64>,
    pub current_order_expected: f64,
    pub gain: f64,
    pub order_exact: bool,
    pub card_order_matters: bool,
    pub approximate: Vec<String>,
}

impl SearchResult {
    pub fn to_json(&self) -> Value {
        let best = self.best.as_ref().map_or(Value::Null, |b| {
            json!({
                "play": b.play,
                "hand": b.hand,
                "scored": b.scored,
                "floor": to_json_number(b.floor),
                "expected": to_json_number(b.expected),
                "ceiling": to_json_number(b.ceiling),
                "blind_pct": to_json_number(b.blind_pct),
            })
        });
        json!({
            "stage": self.stage,
            "done": self.done,
            "best": best,
            "joker_order": self.joker_order,
            "current_order_expected": to_json_number(self.current_order_expected),
            "gain": to_json_number(self.gain),
            "order_exact": self.order_exact,
            "card_order_matters": self.card_order_matters,
            "approximate": self.approximate,
        })
    }
}

/// A candidate kept between stages.
#[derive(Clone)]
struct Candidate {
    play: Vec<usize>,
    order: Vec<usize>,
    expected: f64,
    floor: f64,
}

/// True when `a` should replace `b`: higher expected score, then higher floor.
fn beats(a_expected: f64, a_floor: f64, b_expected: f64, b_floor: f64) -> bool {
    if a_expected.is_nan() {
        return false;
    }
    if b_expected.is_nan() {
        return true;
    }
    if a_expected == b_expected || (a_expected - b_expected).abs() <= EPS * b_expected.abs() {
        return a_floor > b_floor + EPS * b_floor.abs();
    }
    a_expected > b_expected
}

/// Every legal selection of 1 to 5 known cards that includes all forced cards.
fn selections(s: &Snapshot) -> Vec<Vec<usize>> {
    let forced: Vec<usize> = (0..s.hand.len()).filter(|&i| s.hand[i].forced).collect();
    let free: Vec<usize> =
        (0..s.hand.len()).filter(|&i| !s.hand[i].forced && !s.hand[i].hidden).collect();
    let mut out = Vec::new();
    let room = 5usize.saturating_sub(forced.len());
    fn walk(
        free: &[usize],
        start: usize,
        room: usize,
        current: &mut Vec<usize>,
        forced: &[usize],
        out: &mut Vec<Vec<usize>>,
    ) {
        if !current.is_empty() || !forced.is_empty() {
            let mut play: Vec<usize> = forced.iter().chain(current.iter()).copied().collect();
            play.sort_unstable();
            out.push(play);
        }
        if room == 0 {
            return;
        }
        for i in start..free.len() {
            current.push(free[i]);
            walk(free, i + 1, room - 1, current, forced, out);
            current.pop();
        }
    }
    walk(&free, 0, room, &mut Vec::new(), &forced, &mut out);
    out
}

/// All permutations of 0..n, starting with the identity.
fn permutations(n: usize) -> Vec<Vec<usize>> {
    fn walk(items: &mut Vec<usize>, k: usize, out: &mut Vec<Vec<usize>>) {
        if k == items.len() {
            out.push(items.clone());
            return;
        }
        for i in k..items.len() {
            items.swap(k, i);
            walk(items, k + 1, out);
            items.swap(k, i);
        }
    }
    let mut items: Vec<usize> = (0..n).collect();
    let mut out = Vec::new();
    walk(&mut items, 0, &mut out);
    out
}

/// Jokers whose position cannot change the score: they only add chips, or do nothing in scoring.
fn order_free(s: &Snapshot) -> Vec<bool> {
    let copies = s.jokers.iter().any(|j| j.is("j_blueprint") || j.is("j_brainstorm"));
    let baseball = s.jokers.iter().any(|j| j.is("j_baseball"));
    s.jokers
        .iter()
        .map(|j| {
            if copies || (baseball && j.rarity == 2) {
                return false;
            }
            if !matches!(j.ed.as_deref(), None | Some("e_foil") | Some("e_negative")) {
                return false;
            }
            if j.hidden || j.debuff {
                return true;
            }
            let key = j.key.as_deref().unwrap_or("");
            j.num("t_chips", 0.0) > 0.0
                || matches!(
                    key,
                    "j_stuntman"
                        | "j_banner"
                        | "j_blue_joker"
                        | "j_stone"
                        | "j_castle"
                        | "j_ice_cream"
                        | "j_credit_card"
                        | "j_marble"
                        | "j_chaos"
                        | "j_delayed_grat"
                        | "j_egg"
                        | "j_burglar"
                        | "j_faceless"
                        | "j_juggler"
                        | "j_drunkard"
                        | "j_golden"
                        | "j_mr_bones"
                        | "j_troubadour"
                        | "j_merry_andy"
                        | "j_invisible"
                        | "j_chicot"
                        | "j_perkeo"
                )
        })
        .collect()
}

/// Permutations worth trying: order-free jokers keep their relative order.
fn joker_orders(s: &Snapshot) -> Vec<Vec<usize>> {
    let free = order_free(s);
    permutations(s.jokers.len())
        .into_iter()
        .filter(|perm| {
            let kept: Vec<usize> = perm.iter().copied().filter(|&i| free[i]).collect();
            kept.windows(2).all(|w| w[0] < w[1])
        })
        .collect()
}

/// Improves an order by swapping pairs until no swap helps.
fn hill_climb(s: &Snapshot, play: &[usize], cancel: &dyn Fn() -> bool) -> (Vec<usize>, f64) {
    let mut order: Vec<usize> = (0..s.jokers.len()).collect();
    let mut best = score_expected(s, play, &order).score;
    let mut improved = true;
    while improved && !cancel() {
        improved = false;
        for a in 0..order.len() {
            for b in a + 1..order.len() {
                order.swap(a, b);
                let value = score_expected(s, play, &order).score;
                if beats(value, 0.0, best, 0.0) {
                    best = value;
                    improved = true;
                } else {
                    order.swap(a, b);
                }
            }
        }
    }
    (order, best)
}

fn top_first(candidates: &mut [Candidate]) {
    candidates.sort_by(|a, b| {
        if beats(a.expected, a.floor, b.expected, b.floor) {
            std::cmp::Ordering::Less
        } else if beats(b.expected, b.floor, a.expected, a.floor) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
}

struct Progress<'a> {
    s: &'a Snapshot,
    current_order_expected: f64,
    order_exact: bool,
    card_order_matters: bool,
    tags: Vec<String>,
}

impl Progress<'_> {
    fn result(&self, stage: u32, done: bool, best: Option<&Candidate>) -> SearchResult {
        let s = self.s;
        let joker_ids: Vec<i64> = s.jokers.iter().map(|j| j.id).collect();
        let Some(c) = best else {
            return SearchResult {
                stage,
                done,
                best: None,
                joker_order: joker_ids,
                current_order_expected: 0.0,
                gain: 0.0,
                order_exact: self.order_exact,
                card_order_matters: false,
                approximate: self.tags.clone(),
            };
        };
        let scored = score_play(s, &c.play, &c.order);
        // What the joker order alone buys, for the same cards in the same order.
        let identity: Vec<usize> = (0..s.jokers.len()).collect();
        let gain = if c.order == identity {
            0.0
        } else {
            (scored.expected - score_expected(s, &c.play, &identity).score).max(0.0)
        };
        let blind_pct =
            if s.blind.target > 0.0 { scored.expected / s.blind.target * 100.0 } else { 0.0 };
        SearchResult {
            stage,
            done,
            best: Some(Best {
                play: c.play.iter().map(|&i| s.hand[i].id).collect(),
                hand: scored.hand,
                scored: scored.scored,
                floor: scored.floor,
                expected: scored.expected,
                ceiling: scored.ceiling,
                blind_pct,
            }),
            joker_order: c.order.iter().map(|&i| s.jokers[i].id).collect(),
            current_order_expected: self.current_order_expected,
            gain,
            order_exact: self.order_exact,
            card_order_matters: self.card_order_matters,
            approximate: self.tags.clone(),
        }
    }
}

/// Runs the search, publishing after each stage. Stops without a final result when `cancel`
/// returns true.
pub fn run(s: &Snapshot, cancel: &dyn Fn() -> bool, publish: &mut dyn FnMut(SearchResult)) {
    let identity: Vec<usize> = (0..s.jokers.len()).collect();
    let mut progress = Progress {
        s,
        current_order_expected: 0.0,
        order_exact: s.jokers.len() <= MAX_EXACT_JOKERS,
        card_order_matters: false,
        tags: approximate_tags(s),
    };

    // Stage 1: every play with the current joker order.
    let mut candidates: Vec<Candidate> = Vec::new();
    for play in selections(s) {
        if cancel() {
            return;
        }
        let r = score_play(s, &play, &identity);
        if r.legal {
            candidates.push(Candidate {
                play,
                order: identity.clone(),
                expected: r.expected,
                floor: r.floor,
            });
        }
    }
    top_first(&mut candidates);
    let Some(first) = candidates.first().cloned() else {
        publish(progress.result(1, true, None));
        return;
    };
    progress.current_order_expected = first.expected;
    publish(progress.result(1, false, Some(&first)));

    // Stage 2: joker orders.
    if s.jokers.len() > 1 {
        if s.jokers.len() <= MAX_EXACT_JOKERS {
            let orders = joker_orders(s);
            let limit = (MAX_ORDER_EVALS / orders.len().max(1)).max(1);
            if candidates.len() > limit {
                progress.order_exact = false;
            }
            for candidate in candidates.iter_mut().take(limit) {
                if cancel() {
                    return;
                }
                for order in orders.iter().skip(1) {
                    let value = score_expected(s, &candidate.play, order).score;
                    if beats(value, 0.0, candidate.expected, 0.0) {
                        candidate.expected = value;
                        candidate.order = order.clone();
                    }
                }
            }
        } else {
            for candidate in candidates.iter_mut().take(TOP_PLAYS) {
                if cancel() {
                    return;
                }
                let (order, value) = hill_climb(s, &candidate.play, cancel);
                if beats(value, 0.0, candidate.expected, 0.0) {
                    candidate.expected = value;
                    candidate.order = order;
                }
            }
        }
        for candidate in candidates.iter_mut() {
            if candidate.order != identity {
                candidate.floor = score_play(s, &candidate.play, &candidate.order).floor;
            }
        }
        top_first(&mut candidates);
        publish(progress.result(2, false, candidates.first()));
    }

    // Stage 3: card order within the top plays.
    let mut best = candidates[0].clone();
    for candidate in candidates.iter().take(TOP_PLAYS) {
        if cancel() {
            return;
        }
        for perm in permutations(candidate.play.len()).iter().skip(1) {
            let play: Vec<usize> = perm.iter().map(|&i| candidate.play[i]).collect();
            let value = score_expected(s, &play, &candidate.order).score;
            if beats(value, 0.0, best.expected, 0.0) {
                best =
                    Candidate { play, order: candidate.order.clone(), expected: value, floor: 0.0 };
                progress.card_order_matters = true;
            }
        }
    }
    if cancel() {
        return;
    }
    publish(progress.result(3, true, Some(&best)));
}
