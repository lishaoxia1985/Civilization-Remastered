use serde::{Deserialize, Serialize};

use super::Name;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Nation {
    pub name: String,
    #[serde(default)]
    leader_name: String,
    #[serde(default)]
    adjective: Vec<String>,
    #[serde(default)]
    start_bias: Vec<String>,
    #[serde(default)]
    preferred_victory_type: String,
    #[serde(default)]
    start_intro_part1: String,
    #[serde(default)]
    start_intro_part2: String,
    #[serde(default)]
    declaring_war: String,
    #[serde(default)]
    attacked: String,
    #[serde(default)]
    defeated: String,
    #[serde(default)]
    introduction: String,
    #[serde(default)]
    neutral_hello: String,
    #[serde(default)]
    hate_hello: String,
    #[serde(default)]
    trade_request: String,
    outer_color: Option<[u8; 3]>,
    inner_color: Option<[u8; 3]>,
    #[serde(default)]
    favored_religion: String,
    #[serde(default)]
    unique_name: String,
    #[serde(default)]
    uniques: Vec<String>,
    #[serde(default)]
    cities: Vec<String>,
    #[serde(default)]
    city_state_type: String,
}

impl Name for Nation {
    fn name(&self) -> String {
        self.name.to_owned()
    }
}
