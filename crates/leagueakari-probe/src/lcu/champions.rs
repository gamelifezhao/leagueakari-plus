use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Default)]
pub struct ChampionCatalog {
    champions: HashMap<i64, ChampionInfo>,
}

#[derive(Debug, Clone)]
pub struct ChampionInfo {
    pub id: i64,
    pub alias: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ChampionSummary {
    id: i64,
    alias: String,
    name: String,
}

impl ChampionCatalog {
    pub fn from_lcu_summary(value: &Value) -> Self {
        let champions = value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|value| serde_json::from_value::<ChampionSummary>(value.clone()).ok())
            .filter(|champion| champion.id > 0)
            .map(|champion| {
                (
                    champion.id,
                    ChampionInfo {
                        id: champion.id,
                        alias: champion.alias,
                        name: champion.name,
                    },
                )
            })
            .collect();

        Self { champions }
    }

    pub fn label(&self, champion_id: i64) -> String {
        self.champions
            .get(&champion_id)
            .map(|champion| {
                if champion.name.is_empty() {
                    format!("{} ({})", champion.alias, champion.id)
                } else {
                    format!("{} ({})", champion.name, champion.id)
                }
            })
            .unwrap_or_else(|| format!("Unknown champion ({champion_id})"))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn builds_catalog_from_lcu_summary() {
        let catalog = ChampionCatalog::from_lcu_summary(&json!([
            { "id": -1, "alias": "None", "name": "None" },
            { "id": 103, "alias": "Ahri", "name": "阿狸" }
        ]));

        assert_eq!(catalog.label(103), "阿狸 (103)");
        assert_eq!(catalog.label(99999), "Unknown champion (99999)");
    }
}
