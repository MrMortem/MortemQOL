use std::fs;
use std::path::Path;

fn rows() -> Vec<(String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/jokers.md");
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("| j_"))
        .map(|l| {
            let cells: Vec<&str> = l.trim_matches('|').split('|').map(str::trim).collect();
            (cells[0].to_string(), cells[1].to_string())
        })
        .collect()
}

fn test_sources() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| {
            let p = e.unwrap().path();
            let name = p.file_name()?.to_str()?.to_string();
            (name.starts_with("jokers_") && name.ends_with(".rs"))
                .then(|| fs::read_to_string(&p).unwrap())
        })
        .collect()
}

#[test]
fn table_has_150_rows() {
    assert_eq!(rows().len(), 150);
}

#[test]
fn every_modeled_joker_has_a_named_test() {
    let sources = test_sources();
    let needle = |k: &str| format!("fn {k}_");
    let missing: Vec<String> = rows()
        .into_iter()
        .filter(|(k, status)| status == "modeled" && !sources.contains(&needle(k)))
        .map(|(k, _)| k)
        .collect();
    assert!(missing.is_empty(), "no test named for: {missing:?}");
}

#[test]
fn every_row_has_a_known_status() {
    for (k, status) in rows() {
        assert!(["modeled", "no-score-effect", "pending"].contains(&status.as_str()), "{k}");
    }
}

#[test]
fn is_known_matches_the_table() {
    for (k, status) in rows() {
        if status != "pending" {
            assert!(mortemqol_engine::jokers::is_known(&k), "{k} should be known");
        }
    }
    assert!(!mortemqol_engine::jokers::is_known("j_modded"));
}

#[test]
fn no_joker_is_pending() {
    let pending: Vec<String> =
        rows().into_iter().filter(|(_, s)| s == "pending").map(|(k, _)| k).collect();
    assert!(pending.is_empty(), "{pending:?}");
}
