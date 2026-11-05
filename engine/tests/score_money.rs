mod common;
use common::{close, j, number_jokers, snap_json, to_snapshot};
use mortemqol_engine::score::{run_with, score_play, Chance, Mode, ModeChance};
use serde_json::{json, Value};

fn scored(v: &Value, play: &[usize]) -> mortemqol_engine::score::Scored {
    let s = to_snapshot(v);
    let order: Vec<usize> = (0..s.jokers.len()).collect();
    score_play(&s, play, &order)
}

#[test]
fn gold_seal_earns_3() {
    let mut v = snap_json("AS");
    v["hand"][0]["seal"] = json!("Gold");
    let r = scored(&v, &[0]);
    close(r.money_floor, 3.0);
    close(r.money_expected, 3.0);
    close(r.money_ceiling, 3.0);
    close(r.expected, 16.0);
}

#[test]
fn no_money_is_zero() {
    let r = scored(&snap_json("AS"), &[0]);
    close(r.money_floor, 0.0);
    close(r.money_expected, 0.0);
    close(r.money_ceiling, 0.0);
}

#[test]
fn lucky_money_range() {
    let mut v = snap_json("AS");
    v["hand"][0]["enh"] = json!("m_lucky");
    let r = scored(&v, &[0]);
    close(r.money_floor, 0.0);
    close(r.money_expected, 20.0 / 15.0);
    close(r.money_ceiling, 20.0);
}

#[test]
fn money_serializes() {
    let mut v = snap_json("AS");
    v["hand"][0]["seal"] = json!("Gold");
    assert_eq!(scored(&v, &[0]).to_json()["money_expected"], json!(3.0));
}

struct AlwaysFires {
    used: bool,
}

impl Chance for AlwaysFires {
    fn weight(&mut self, _p: f64) -> f64 {
        self.used = true;
        1.0
    }
    fn pick(&mut self, _min: f64, max: f64) -> f64 {
        self.used = true;
        max
    }
    fn used(&self) -> bool {
        self.used
    }
}

#[test]
fn custom_chance_drives_joker_effects() {
    let mut v = snap_json("AH");
    v["hand"][0]["enh"] = json!("m_lucky");
    v["jokers"] = Value::Array(number_jokers(vec![
        j("j_bloodstone", json!({"extra": {"odds": 2, "Xmult": 1.5}})),
        j("j_misprint", json!({"extra": {"min": 0, "max": 23}})),
    ]));
    let s = to_snapshot(&v);
    let mut custom = AlwaysFires { used: false };
    let got = run_with(&s, &[0], &[0, 1], &mut custom);
    let mut ceiling = ModeChance::new(Mode::Ceiling);
    let want = run_with(&s, &[0], &[0, 1], &mut ceiling);
    close(got.score, want.score);
    close(got.money, want.money);
    assert!(custom.used());
    close(got.score, scored(&v, &[0]).ceiling);
}

/// Plays every card but the last `held`, with this balance and these jokers.
fn money(
    hand: &str,
    held: usize,
    jokers: Vec<Value>,
    f: impl Fn(&mut Value),
) -> mortemqol_engine::score::Scored {
    let mut v = snap_json(hand);
    v["jokers"] = Value::Array(number_jokers(jokers));
    f(&mut v);
    let s = to_snapshot(&v);
    let play: Vec<usize> = (0..s.hand.len() - held).collect();
    let order: Vec<usize> = (0..s.jokers.len()).collect();
    score_play(&s, &play, &order)
}

fn exact(r: &mortemqol_engine::score::Scored, want: f64) {
    close(r.money_floor, want);
    close(r.money_expected, want);
    close(r.money_ceiling, want);
}

#[test]
fn j_ticket_money() {
    let r = money("AS", 0, vec![j("j_ticket", json!({"extra": 4}))], |v| {
        v["hand"][0]["enh"] = json!("m_gold")
    });
    exact(&r, 4.0);
}

#[test]
fn j_rough_gem_money() {
    exact(&money("AD", 0, vec![j("j_rough_gem", json!({"extra": 1}))], |_| {}), 1.0);
}

#[test]
fn j_business_money_range() {
    let r = money("KS", 0, vec![j("j_business", json!({"extra": 2}))], |_| {});
    close(r.money_floor, 0.0);
    close(r.money_expected, 1.0);
    close(r.money_ceiling, 2.0);
}

#[test]
fn j_reserved_parking_money_range() {
    let parking = j("j_reserved_parking", json!({"extra": {"odds": 2, "dollars": 1}}));
    let r = money("AS KH", 1, vec![parking], |_| {});
    close(r.money_floor, 0.0);
    close(r.money_expected, 0.5);
    close(r.money_ceiling, 1.0);
}

#[test]
fn j_todo_list_money() {
    let todo =
        || j("j_todo_list", json!({"extra": {"dollars": 4}, "to_do_poker_hand": "High Card"}));
    exact(&money("AS", 0, vec![todo()], |_| {}), 4.0);
    exact(&money("AS AH", 0, vec![todo()], |_| {}), 0.0);
}

#[test]
fn gold_seal_retrigger_money() {
    let mut v = snap_json("AS");
    v["hand"][0]["seal"] = json!("Gold");
    v["jokers"] = Value::Array(number_jokers(vec![j("j_selzer", json!({"extra": 10}))]));
    exact(&scored(&v, &[0]), 6.0);
}

#[test]
fn blueprint_rough_gem_money() {
    let jokers = vec![j("j_blueprint", json!({})), j("j_rough_gem", json!({"extra": 1}))];
    exact(&money("AD", 0, jokers, |_| {}), 2.0);
}

#[test]
fn tooth_money() {
    let r = money("2S 3H 7D 9C JS", 0, vec![], |v| v["blind"]["key"] = json!("bl_tooth"));
    exact(&r, -5.0);
}

fn ox(v: &mut Value, balance: f64) {
    v["blind"]["key"] = json!("bl_ox");
    v["round"]["most_played"] = json!("High Card");
    v["round"]["dollars"] = json!(balance);
}

#[test]
fn ox_takes_balance() {
    exact(&money("AS", 0, vec![], |v| ox(v, 10.0)), -10.0);
}

#[test]
fn ox_with_gold_seal() {
    let r = money("AS", 0, vec![], |v| {
        ox(v, 10.0);
        v["hand"][0]["seal"] = json!("Gold");
    });
    exact(&r, -7.0);
}

#[test]
fn ox_with_debt_brings_balance_to_zero() {
    exact(&money("AS", 0, vec![], |v| ox(v, -5.0)), 5.0);
}

#[test]
fn ox_at_zero_costs_nothing() {
    exact(&money("AS", 0, vec![], |v| ox(v, 0.0)), 0.0);
}

#[test]
fn j_bull_with_debt_adds_nothing() {
    let r = money("AS", 0, vec![j("j_bull", json!({"extra": 2}))], |v| {
        v["round"]["dollars"] = json!(-5)
    });
    close(r.expected, 16.0);
}

#[test]
fn matador_on_debuffed_card() {
    let r = money("AS", 0, vec![j("j_matador", json!({"extra": 8}))], |v| {
        v["hand"][0]["debuff"] = json!(true)
    });
    exact(&r, 8.0);
}

fn psychic(v: &mut Value) {
    v["blind"]["key"] = json!("bl_psychic");
}

#[test]
fn blocked_matador_pays() {
    let r = money("AS", 0, vec![j("j_matador", json!({"extra": 8}))], psychic);
    exact(&r, 8.0);
    close(r.expected, 0.0);
    assert!(!r.legal);
}

#[test]
fn blocked_copied_matador_pays_twice() {
    let jokers = vec![j("j_blueprint", json!({})), j("j_matador", json!({"extra": 8}))];
    exact(&money("AS", 0, jokers, psychic), 16.0);
}

#[test]
fn blocked_todo_list_pays_nothing() {
    let todo = j("j_todo_list", json!({"extra": {"dollars": 4}, "to_do_poker_hand": "High Card"}));
    exact(&money("AS", 0, vec![todo], psychic), 0.0);
}

#[test]
fn blocked_without_matador_is_zero() {
    exact(&money("AS", 0, vec![], psychic), 0.0);
}

#[test]
fn blocked_expected_money_reaches_the_search_path() {
    let mut v = snap_json("AS");
    v["jokers"] = Value::Array(number_jokers(vec![j("j_matador", json!({"extra": 8}))]));
    psychic(&mut v);
    let got = mortemqol_engine::score::score_expected(&to_snapshot(&v), &[0], &[0]);
    close(got.money, 8.0);
    close(got.score, 0.0);
}
