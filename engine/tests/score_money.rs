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
