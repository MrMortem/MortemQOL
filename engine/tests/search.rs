mod common;
use common::{close, j, number_jokers, snap_json, table, to_snapshot};
use mortemqol_engine::score::score_play;
use mortemqol_engine::search::{run, SearchResult};
use mortemqol_engine::snapshot::Snapshot;
use mortemqol_engine::worker::{self, PollStatus};
use serde_json::{json, Value};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The worker is global, so tests that use it take turns.
static WORKER: Mutex<()> = Mutex::new(());

fn search(s: &Snapshot) -> Vec<SearchResult> {
    let mut out = Vec::new();
    run(s, &|| false, &mut |r| out.push(r));
    out
}

fn last(s: &Snapshot) -> SearchResult {
    search(s).pop().unwrap()
}

fn with_jokers(hand: &str, jokers: Vec<Value>) -> Value {
    let mut v = snap_json(hand);
    v["jokers"] = Value::Array(number_jokers(jokers));
    v
}

#[test]
fn picks_pair_over_high_card() {
    let r = last(&to_snapshot(&snap_json("AS AH 3D")));
    let best = r.best.unwrap();
    assert_eq!(best.hand, "Pair");
    close(best.expected, 64.0);
    assert!(r.done);
}

#[test]
fn finds_joker_reorder_gain() {
    let v = with_jokers(
        "AS",
        vec![j("j_cavendish", json!({"extra": {"Xmult": 3}})), j("j_joker", json!({"mult": 4}))],
    );
    let r = last(&to_snapshot(&v));
    close(r.current_order_expected, 112.0);
    close(r.best.unwrap().expected, 240.0);
    close(r.gain, 128.0);
    assert_eq!(r.joker_order, vec![101, 100]);
    assert!(r.order_exact);
}

#[test]
fn no_gain_keeps_current_joker_order() {
    let v = with_jokers(
        "AS",
        vec![j("j_joker", json!({"mult": 4})), j("j_red_card", json!({"mult": 6}))],
    );
    let r = last(&to_snapshot(&v));
    close(r.gain, 0.0);
    assert_eq!(r.joker_order, vec![100, 101]);
}

#[test]
fn finds_card_order() {
    let mut v = with_jokers("KS KH", vec![j("j_photograph", json!({"extra": 2}))]);
    v["hand"][1]["enh"] = json!("m_mult");
    let r = last(&to_snapshot(&v));
    let best = r.best.unwrap();
    close(best.expected, 360.0);
    assert_eq!(best.play, vec![2, 1]);
    assert!(r.card_order_matters);
}

#[test]
fn card_order_gain_is_not_reported_as_joker_gain() {
    let mut v = with_jokers(
        "AS AH",
        vec![
            j("j_joker", json!({"mult": 4})),
            j("j_stuntman", json!({"extra": {"chip_mod": 250}})),
        ],
    );
    v["hand"][0]["enh"] = json!("m_glass");
    v["hand"][1]["enh"] = json!("m_mult");
    let r = last(&to_snapshot(&v));
    assert!(r.card_order_matters);
    close(r.gain, 0.0);
    assert_eq!(r.joker_order, vec![100, 101]);
}

#[test]
fn joker_gain_excludes_card_order_when_both_help() {
    let mut v = with_jokers(
        "AS AH",
        vec![j("j_cavendish", json!({"extra": {"Xmult": 3}})), j("j_joker", json!({"mult": 4}))],
    );
    v["hand"][0]["enh"] = json!("m_glass");
    v["hand"][1]["enh"] = json!("m_mult");
    let r = last(&to_snapshot(&v));
    let best = r.best.clone().unwrap();
    assert!(r.card_order_matters);
    // Pair chips 32, recommended cards mult then glass: (2 + 4) X2 = 12.
    // Joker then Cavendish: (12 + 4) X3 = 48. Cavendish then Joker: 12 X3 + 4 = 40.
    assert_eq!(best.play, vec![2, 1]);
    close(best.expected, 32.0 * 48.0);
    close(r.gain, 32.0 * 8.0);
}

#[test]
fn card_order_flag_off_when_order_is_irrelevant() {
    assert!(!last(&to_snapshot(&snap_json("KS KH"))).card_order_matters);
}

#[test]
fn never_below_any_direct_score() {
    for s in table() {
        let best = last(&s).best.unwrap().expected;
        let order: Vec<usize> = (0..s.jokers.len()).collect();
        let n = s.hand.len();
        for mask in 1u32..(1 << n) {
            if mask.count_ones() > 5 {
                continue;
            }
            let play: Vec<usize> = (0..n).filter(|i| mask >> i & 1 == 1).collect();
            let direct = score_play(&s, &play, &order).expected;
            assert!(best + 1e-6 >= direct, "{best} < {direct}");
        }
    }
}

#[test]
fn forced_card_always_in_play() {
    let mut v = snap_json("AS AH 3D");
    v["hand"][2]["forced"] = json!(true);
    let best = last(&to_snapshot(&v)).best.unwrap();
    assert!(best.play.contains(&3));
}

#[test]
fn hidden_cards_never_chosen() {
    let mut v = snap_json("AS 3D");
    v["hand"][0] = json!({"id": 1, "hidden": true});
    let r = last(&to_snapshot(&v));
    assert_eq!(r.best.unwrap().play, vec![2]);
    assert!(r.approximate.contains(&"hidden_card".to_string()));
}

#[test]
fn empty_hand_gives_null_best() {
    let r = last(&to_snapshot(&snap_json("")));
    assert!(r.best.is_none());
    assert!(r.done);
}

#[test]
fn one_card_hand_works() {
    close(last(&to_snapshot(&snap_json("AS"))).best.unwrap().expected, 16.0);
}

#[test]
fn all_plays_illegal_gives_null_best() {
    let mut v = snap_json("AS AH");
    v["blind"]["key"] = json!("bl_psychic");
    assert!(last(&to_snapshot(&v)).best.is_none());
}

#[test]
fn over_6_jokers_sets_order_exact_false() {
    let jokers = (0..7).map(|_| j("j_joker", json!({"mult": 4}))).collect();
    let r = last(&to_snapshot(&with_jokers("AS", jokers)));
    assert!(!r.order_exact);
}

#[test]
fn equal_expected_prefers_higher_floor() {
    // Lucky AS expects 16 * 5 = 80. Plain 3H with Mult card and Bonus is 80 flat.
    let mut v = snap_json("AS 3H");
    v["hand"][0]["enh"] = json!("m_lucky");
    v["hand"][1]["perma"] = json!({"bonus": 72});
    let best = last(&to_snapshot(&v)).best.unwrap();
    close(best.expected, 80.0);
    assert_eq!(best.play, vec![2]);
    close(best.floor, 80.0);
}

#[test]
fn blind_pct_uses_target() {
    let best = last(&to_snapshot(&snap_json("AS AH"))).best.unwrap();
    close(best.blind_pct, 64.0 / 300.0 * 100.0);
}

#[test]
fn results_report_stage_progression() {
    let v = with_jokers(
        "AS AH",
        vec![j("j_joker", json!({"mult": 4})), j("j_red_card", json!({"mult": 6}))],
    );
    let results = search(&to_snapshot(&v));
    let stages: Vec<u32> = results.iter().map(|r| r.stage).collect();
    assert!(stages.windows(2).all(|w| w[0] < w[1]), "{stages:?}");
    assert!(results.iter().rev().skip(1).all(|r| !r.done));
    assert!(results.last().unwrap().done);
}

#[test]
fn cancel_stops_without_final_result() {
    let mut out = Vec::new();
    run(&to_snapshot(&snap_json("AS AH")), &|| true, &mut |r| out.push(r));
    assert!(out.iter().all(|r| !r.done));
}

#[test]
fn naneinf_result_serializes() {
    let mut v = snap_json("AS");
    v["levels"]["High Card"]["chips"] = json!(1e308);
    v["levels"]["High Card"]["mult"] = json!(1e308);
    let out = last(&to_snapshot(&v)).to_json();
    assert_eq!(out["best"]["expected"], json!("naneinf"));
}

fn money_mode(v: &mut Value) {
    v["options"] = json!({"objective": "money"});
}

fn gold(v: &mut Value, index: usize) {
    v["hand"][index]["seal"] = json!("Gold");
}

#[test]
fn money_mode_prefers_money_over_score() {
    let mut v = snap_json("AS AH 2D");
    gold(&mut v, 2);
    money_mode(&mut v);
    let best = last(&to_snapshot(&v)).best.unwrap();
    close(best.money_expected, 3.0);
    assert_eq!(best.play, vec![3]);
    close(best.expected, 7.0);
}

#[test]
fn money_ties_go_to_score() {
    let mut v = snap_json("2D 3H");
    gold(&mut v, 0);
    gold(&mut v, 1);
    money_mode(&mut v);
    let best = last(&to_snapshot(&v)).best.unwrap();
    close(best.money_expected, 3.0);
    close(best.expected, 8.0);
}

#[test]
fn no_money_matches_score_mode() {
    let mut compared = 0;
    for s in table() {
        if s.hand.iter().any(|c| c.has_enh("m_lucky")) {
            continue;
        }
        let score = last(&s).best.unwrap();
        let mut money = s.clone();
        money.options.objective = mortemqol_engine::snapshot::Objective::Money;
        let best = last(&money).best.unwrap();
        close(best.expected, score.expected);
        assert_eq!(best.play, score.play);
        compared += 1;
    }
    assert!(compared > 0);
}

#[test]
fn score_mode_ignores_money() {
    let mut v = snap_json("AS AH 2D");
    gold(&mut v, 2);
    let best = last(&to_snapshot(&v)).best.unwrap();
    assert_eq!(best.hand, "Pair");
    close(best.expected, 64.0);
}

fn psychic_matador() -> Value {
    let mut v = with_jokers("AS AH", vec![j("j_matador", json!({"extra": 8}))]);
    v["blind"]["key"] = json!("bl_psychic");
    v
}

#[test]
fn blocked_never_recommended_in_score_mode() {
    assert!(last(&to_snapshot(&psychic_matador())).best.is_none());
}

#[test]
fn blocked_recommended_in_money_mode() {
    let mut v = psychic_matador();
    money_mode(&mut v);
    let r = last(&to_snapshot(&v));
    assert!(r.blocked);
    let best = r.best.unwrap();
    close(best.money_expected, 8.0);
    close(best.expected, 0.0);
}

#[test]
fn blocked_zero_money_is_not_recommended() {
    let mut v = snap_json("AS AH");
    v["blind"]["key"] = json!("bl_psychic");
    money_mode(&mut v);
    assert!(last(&to_snapshot(&v)).best.is_none());
}

#[test]
fn money_joker_order_search() {
    let mut v = with_jokers(
        "AD",
        vec![
            j("j_blueprint", json!({})),
            j("j_joker", json!({"mult": 4})),
            j("j_rough_gem", json!({"extra": 1})),
        ],
    );
    money_mode(&mut v);
    let r = last(&to_snapshot(&v));
    close(r.best.clone().unwrap().money_expected, 2.0);
    close(r.gain_money, 1.0);
}

#[test]
fn money_card_order_search() {
    let mut v = with_jokers("AS AH", vec![j("j_hanging_chad", json!({"extra": 2}))]);
    gold(&mut v, 1);
    money_mode(&mut v);
    let r = last(&to_snapshot(&v));
    let best = r.best.unwrap();
    close(best.money_expected, 9.0);
    assert_eq!(best.play, vec![2, 1]);
    assert!(r.card_order_matters);
}

fn last_hand(v: &mut Value) {
    money_mode(v);
    v["round"]["hands_left"] = json!(1);
}

#[test]
fn blind_warning_on_losing_last_hand() {
    let mut v = snap_json("AS");
    last_hand(&mut v);
    assert!(last(&to_snapshot(&v)).blind_warning);
}

#[test]
fn blind_warning_off_when_hands_remain() {
    let mut v = snap_json("AS");
    money_mode(&mut v);
    assert!(!last(&to_snapshot(&v)).blind_warning);
}

#[test]
fn blind_warning_off_when_it_wins() {
    let mut v = snap_json("AS");
    last_hand(&mut v);
    v["round"]["chips"] = json!(290);
    assert!(!last(&to_snapshot(&v)).blind_warning);
}

#[test]
fn blind_warning_off_in_score_mode() {
    let mut v = snap_json("AS");
    v["round"]["hands_left"] = json!(1);
    assert!(!last(&to_snapshot(&v)).blind_warning);
}

#[test]
fn sort_handles_infinite_scores() {
    let mut v = snap_json("AS AH KD KC 2S 3H 7D 9C");
    v["levels"]["Pair"]["chips"] = json!(1e308);
    v["levels"]["Pair"]["mult"] = json!(1e308);
    v["levels"]["Two Pair"]["chips"] = json!(1e308);
    v["levels"]["Two Pair"]["mult"] = json!(1e308);
    let r = last(&to_snapshot(&v));
    assert!(r.done);
    assert!(r.best.unwrap().expected.is_infinite());
}

fn wait_done(id: u64) -> String {
    let start = Instant::now();
    let mut latest = String::new();
    loop {
        let (status, data) = worker::poll(id);
        if let Some(d) = data {
            latest = String::from_utf8(d).unwrap();
        }
        match status {
            PollStatus::Finished => return latest,
            PollStatus::Running => {}
            other => panic!("unexpected {other:?}: {latest}"),
        }
        assert!(start.elapsed() < Duration::from_secs(20), "timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn worker_finishes_with_json() {
    let _guard = WORKER.lock().unwrap();
    let id = worker::start(snap_json("AS AH").to_string().into_bytes());
    let out: Value = serde_json::from_str(&wait_done(id)).unwrap();
    assert_eq!(out["best"]["hand"], json!("Pair"));
    assert_eq!(out["done"], json!(true));
}

#[test]
fn worker_reports_malformed_snapshot_as_error() {
    let _guard = WORKER.lock().unwrap();
    let id = worker::start(b"{not json".to_vec());
    let start = Instant::now();
    loop {
        let (status, _) = worker::poll(id);
        if matches!(status, PollStatus::Error) {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn large_snapshot() -> String {
    let hand = "AS KS QS JS 10S 9S 8S 7S 6S 5S 4S 3S 2S AH KH";
    let jokers = (0..8).map(|_| j("j_cavendish", json!({"extra": {"Xmult": 3}}))).collect();
    with_jokers(hand, jokers).to_string()
}

#[test]
fn new_search_supersedes_old() {
    let _guard = WORKER.lock().unwrap();
    let first = worker::start(large_snapshot().into_bytes());
    let second = worker::start(snap_json("AS").to_string().into_bytes());
    assert_eq!(worker::poll(first).0, PollStatus::Unknown);
    wait_done(second);
}

#[test]
fn large_search_cancels_quickly() {
    let _guard = WORKER.lock().unwrap();
    worker::start(large_snapshot().into_bytes());
    std::thread::sleep(Duration::from_millis(5));
    let begun = Instant::now();
    let id = worker::start(snap_json("AS").to_string().into_bytes());
    wait_done(id);
    assert!(begun.elapsed() < Duration::from_millis(200), "{:?}", begun.elapsed());
}

#[test]
fn unknown_id_is_unknown() {
    assert_eq!(worker::poll(u64::MAX).0, PollStatus::Unknown);
}

#[test]
fn ffi_search_round_trip() {
    let _guard = WORKER.lock().unwrap();
    use mortemqol_engine::ffi::*;
    let json = snap_json("AS AH").to_string();
    let id = unsafe { mqol_search_start(json.as_ptr(), json.len()) };
    assert!(id >= 1);
    let mut small = [0u8; 4];
    let mut big = vec![0u8; 8192];
    let mut len = 0usize;
    let start = Instant::now();
    loop {
        let status = unsafe { mqol_search_poll(id, small.as_mut_ptr(), small.len(), &mut len) };
        if status == STATUS_TOO_SMALL {
            let again = unsafe { mqol_search_poll(id, big.as_mut_ptr(), big.len(), &mut len) };
            if again == STATUS_FINISHED {
                assert!(len > 0);
                let out: Value = serde_json::from_slice(&big[..len]).unwrap();
                assert_eq!(out["done"], json!(true));
                return;
            }
        }
        assert!(start.elapsed() < Duration::from_secs(5));
    }
}
