//! The visible game state sent by the Lua layer. See the plan's Shared Contracts.
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub const SCHEMA: u32 = 1;

fn one() -> f64 {
    1.0
}

#[derive(Debug, Clone, Deserialize)]
pub struct Perma {
    #[serde(default)]
    pub bonus: f64,
    #[serde(default)]
    pub mult: f64,
    #[serde(default = "one")]
    pub x_mult: f64,
}

impl Default for Perma {
    fn default() -> Self {
        Perma { bonus: 0.0, mult: 0.0, x_mult: 1.0 }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Card {
    pub id: i64,
    /// 2 to 14. Zero when hidden.
    #[serde(default)]
    pub rank: i32,
    #[serde(default)]
    pub suit: String,
    #[serde(default)]
    pub enh: Option<String>,
    #[serde(default)]
    pub ed: Option<String>,
    #[serde(default)]
    pub seal: Option<String>,
    #[serde(default)]
    pub perma: Perma,
    #[serde(default)]
    pub debuff: bool,
    #[serde(default)]
    pub forced: bool,
    #[serde(default)]
    pub hidden: bool,
}

impl Card {
    pub fn has_enh(&self, key: &str) -> bool {
        self.enh.as_deref() == Some(key)
    }
    pub fn is_stone(&self) -> bool {
        self.has_enh("m_stone")
    }
    pub fn is_seal(&self, seal: &str) -> bool {
        self.seal.as_deref() == Some(seal)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Joker {
    pub id: i64,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub ed: Option<String>,
    #[serde(default)]
    pub debuff: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub sell: f64,
    #[serde(default)]
    pub ab: Value,
}

impl Joker {
    pub fn is(&self, key: &str) -> bool {
        self.key.as_deref() == Some(key)
    }
    /// Reads a number from `ability`, falling back when missing or not numeric.
    pub fn num(&self, name: &str, default: f64) -> f64 {
        self.ab.get(name).and_then(Value::as_f64).unwrap_or(default)
    }
    /// Reads a number from `ability.extra`, falling back when missing or not numeric.
    pub fn extra_num(&self, name: &str, default: f64) -> f64 {
        self.ab.get("extra").and_then(|e| e.get(name)).and_then(Value::as_f64).unwrap_or(default)
    }
    /// Reads `ability.extra` itself as a number.
    pub fn extra(&self, default: f64) -> f64 {
        self.ab.get("extra").and_then(Value::as_f64).unwrap_or(default)
    }
    /// Reads a string from `ability.extra`.
    pub fn extra_str(&self, name: &str) -> Option<&str> {
        self.ab.get("extra").and_then(|e| e.get(name)).and_then(Value::as_str)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Level {
    #[serde(default = "one")]
    pub level: f64,
    pub chips: f64,
    pub mult: f64,
    #[serde(default)]
    pub l_chips: f64,
    #[serde(default)]
    pub l_mult: f64,
    #[serde(default)]
    pub played: f64,
    #[serde(default)]
    pub played_round: f64,
    #[serde(default)]
    pub visible: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Blind {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub target: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Round {
    #[serde(default)]
    pub hands_left: f64,
    #[serde(default)]
    pub discards_left: f64,
    #[serde(default)]
    pub dollars: f64,
    #[serde(default)]
    pub chips: f64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Deck {
    #[serde(default)]
    pub back: String,
    #[serde(default)]
    pub remaining: f64,
    #[serde(default)]
    pub total: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Consumable {
    #[serde(default)]
    pub key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Snapshot {
    pub schema: u32,
    #[serde(default)]
    pub hand: Vec<Card>,
    #[serde(default)]
    pub jokers: Vec<Joker>,
    #[serde(default)]
    pub levels: BTreeMap<String, Level>,
    #[serde(default)]
    pub blind: Blind,
    #[serde(default)]
    pub round: Round,
    #[serde(default)]
    pub deck: Deck,
    #[serde(default)]
    pub vouchers: Vec<String>,
    #[serde(default)]
    pub consumables: Vec<Consumable>,
    #[serde(default)]
    pub lobby: bool,
}

pub fn parse(json: &[u8]) -> Result<Snapshot, String> {
    let s: Snapshot = serde_json::from_slice(json).map_err(|e| e.to_string())?;
    if s.schema != SCHEMA {
        return Err(format!("unsupported snapshot schema {}", s.schema));
    }
    Ok(s)
}
