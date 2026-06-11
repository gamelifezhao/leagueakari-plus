use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::models::DraftState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionAnalysis {
    pub confidence: AnalysisConfidence,
    pub dimensions: CompositionDimensions,
    pub strengths: Vec<String>,
    pub risks: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompositionDimensions {
    pub engage: u8,
    pub frontline: u8,
    pub magic_damage: u8,
    pub physical_damage: u8,
    pub crowd_control: u8,
    pub scaling: u8,
}

const SAMPLE_TAGS_JSON: &str = include_str!("../../data/champion-tags.sample.json");

#[derive(Debug, Clone, Default, Deserialize)]
struct ChampionTags {
    champion_id: i64,
    #[serde(default)]
    engage: u8,
    #[serde(default)]
    frontline: u8,
    #[serde(default)]
    magic_damage: u8,
    #[serde(default)]
    physical_damage: u8,
    #[serde(default)]
    crowd_control: u8,
    #[serde(default)]
    scaling: u8,
}

pub fn analyze_draft(draft: &DraftState) -> CompositionAnalysis {
    let tag_db = sample_tags();
    let picked_champions = draft
        .my_team
        .iter()
        .filter_map(|player| player.champion_id)
        .collect::<Vec<_>>();
    let known_tags = picked_champions
        .iter()
        .filter_map(|champion_id| tag_db.get(champion_id))
        .collect::<Vec<_>>();
    let dimensions = aggregate_dimensions(&known_tags);
    let confidence = confidence_for(picked_champions.len(), known_tags.len());
    let strengths = strengths_for(&dimensions);
    let risks = risks_for(&dimensions);
    let suggestions = suggestions_for(&dimensions);

    CompositionAnalysis {
        confidence,
        dimensions,
        strengths,
        risks,
        suggestions,
    }
}

fn aggregate_dimensions(tags: &[&ChampionTags]) -> CompositionDimensions {
    CompositionDimensions {
        engage: team_dimension(tags.iter().map(|tags| tags.engage)),
        frontline: team_dimension(tags.iter().map(|tags| tags.frontline)),
        magic_damage: team_dimension(tags.iter().map(|tags| tags.magic_damage)),
        physical_damage: team_dimension(tags.iter().map(|tags| tags.physical_damage)),
        crowd_control: team_dimension(tags.iter().map(|tags| tags.crowd_control)),
        scaling: team_dimension(tags.iter().map(|tags| tags.scaling)),
    }
}

fn team_dimension(values: impl Iterator<Item = u8>) -> u8 {
    values.map(|value| value as u16).sum::<u16>().min(100) as u8
}

fn confidence_for(picked_count: usize, known_count: usize) -> AnalysisConfidence {
    if picked_count < 3 || known_count < 2 {
        AnalysisConfidence::Low
    } else if picked_count >= 5 && known_count >= 4 {
        AnalysisConfidence::High
    } else {
        AnalysisConfidence::Medium
    }
}

fn strengths_for(dimensions: &CompositionDimensions) -> Vec<String> {
    let mut strengths = Vec::new();

    if dimensions.engage >= 65 {
        strengths.push("我方具备较好的主动开团能力。".to_string());
    }
    if dimensions.frontline >= 65 {
        strengths.push("我方前排厚度较好，团战容错更高。".to_string());
    }
    if dimensions.crowd_control >= 65 {
        strengths.push("我方控制链较充足，容易配合抓机会。".to_string());
    }
    if dimensions.scaling >= 65 {
        strengths.push("我方后期成长性不错，适合稳住资源节奏。".to_string());
    }

    strengths
}

fn risks_for(dimensions: &CompositionDimensions) -> Vec<String> {
    let mut risks = Vec::new();

    if dimensions.frontline <= 35 {
        risks.push("我方前排偏少，正面团战可能缺少承伤点。".to_string());
    }
    if dimensions.engage <= 35 {
        risks.push("我方主动开团偏弱，可能需要靠反打或视野抓人。".to_string());
    }
    if dimensions.magic_damage <= 30 {
        risks.push("我方魔法伤害偏低，敌方可能更容易堆护甲。".to_string());
    }
    if dimensions.physical_damage <= 30 {
        risks.push("我方物理伤害偏低，持续输出结构可能不够均衡。".to_string());
    }

    risks
}

fn suggestions_for(dimensions: &CompositionDimensions) -> Vec<String> {
    let mut suggestions = Vec::new();

    if dimensions.frontline <= 35 {
        suggestions.push("后续选人优先补一个能站前排的英雄。".to_string());
    }
    if dimensions.engage <= 35 {
        suggestions.push("后续选人可以考虑补主动开团或强先手。".to_string());
    }
    if dimensions.magic_damage <= 30 {
        suggestions.push("后续选人尽量补充 AP 伤害。".to_string());
    }
    if dimensions.crowd_control <= 35 {
        suggestions.push("后续选人可以补稳定控制，提升抓机会能力。".to_string());
    }

    suggestions
}

fn sample_tags() -> HashMap<i64, ChampionTags> {
    serde_json::from_str::<Vec<ChampionTags>>(SAMPLE_TAGS_JSON)
        .unwrap_or_default()
        .into_iter()
        .map(|tags| (tags.champion_id, tags))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lcu::models::DraftPlayer;

    #[test]
    fn returns_low_confidence_for_empty_draft() {
        let analysis = analyze_draft(&DraftState::empty("ChampSelect".to_string()));

        assert_eq!(analysis.confidence, AnalysisConfidence::Low);
        assert!(!analysis.suggestions.is_empty());
    }

    #[test]
    fn loads_sample_tags_from_json() {
        let tags = sample_tags();

        assert!(tags.contains_key(&103));
        assert!(tags.contains_key(&22));
    }

    #[test]
    fn detects_frontline_and_engage_strengths() {
        let mut draft = DraftState::empty("ChampSelect".to_string());
        draft.my_team = vec![player(54), player(89), player(103), player(22), player(104)];

        let analysis = analyze_draft(&draft);

        assert_eq!(analysis.confidence, AnalysisConfidence::High);
        assert!(analysis.dimensions.engage >= 45);
        assert!(!analysis.strengths.is_empty());
    }

    fn player(champion_id: i64) -> DraftPlayer {
        DraftPlayer {
            cell_id: champion_id,
            champion_id: Some(champion_id),
            assigned_position: None,
            summoner_id: None,
        }
    }
}
