use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::models::DraftState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionAnalysis {
    pub confidence: AnalysisConfidence,
    pub dimensions: CompositionDimensions,
    pub enemy_dimensions: CompositionDimensions,
    pub strengths: Vec<String>,
    pub risks: Vec<String>,
    pub enemy_threats: Vec<String>,
    pub win_conditions: Vec<String>,
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
    let enemy_champions = draft
        .their_team
        .iter()
        .filter_map(|player| player.champion_id)
        .collect::<Vec<_>>();
    let known_tags = picked_champions
        .iter()
        .filter_map(|champion_id| tag_db.get(champion_id))
        .collect::<Vec<_>>();
    let enemy_known_tags = enemy_champions
        .iter()
        .filter_map(|champion_id| tag_db.get(champion_id))
        .collect::<Vec<_>>();
    let dimensions = aggregate_dimensions(&known_tags);
    let enemy_dimensions = aggregate_dimensions(&enemy_known_tags);
    let confidence = confidence_for(
        picked_champions.len() + enemy_champions.len(),
        known_tags.len() + enemy_known_tags.len(),
    );
    let strengths = strengths_for(&dimensions);
    let mut risks = risks_for(&dimensions);
    risks.extend(matchup_risks_for(&dimensions, &enemy_dimensions));
    let enemy_threats = enemy_threats_for(&dimensions, &enemy_dimensions);
    let win_conditions = win_conditions_for(&dimensions, &enemy_dimensions);
    let mut suggestions = suggestions_for(&dimensions, picked_champions.len() >= 5);
    suggestions.extend(counterplay_suggestions_for(&dimensions, &enemy_dimensions));

    CompositionAnalysis {
        confidence,
        dimensions,
        enemy_dimensions,
        strengths,
        risks,
        enemy_threats,
        win_conditions,
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

fn matchup_risks_for(my: &CompositionDimensions, enemy: &CompositionDimensions) -> Vec<String> {
    let mut risks = Vec::new();

    if enemy.engage >= 75 && enemy.crowd_control >= 65 {
        risks.push("敌方强开和控制链很强，站位过密时容易被连续进场。".to_string());
    }
    if enemy.frontline >= 75 && my.magic_damage <= 40 {
        risks.push("敌方前排厚度高，而我方 AP 压力不足，正面团可能打坦速度偏慢。".to_string());
    }
    if enemy.magic_damage >= 65 && enemy.physical_damage >= 65 {
        risks.push("敌方伤害类型比较混合，单一抗性装备收益会下降。".to_string());
    }
    if enemy.scaling >= 75 && my.scaling < enemy.scaling {
        risks.push("敌方后期成长性更高，拖到大后期会越来越难处理。".to_string());
    }

    risks
}

fn enemy_threats_for(my: &CompositionDimensions, enemy: &CompositionDimensions) -> Vec<String> {
    let mut threats = Vec::new();

    if enemy.engage >= 75 {
        threats.push("敌方具备强先手，第一波开团会决定很多团战结果。".to_string());
    }
    if enemy.frontline >= 75 {
        threats.push("敌方前排较硬，正面阵地战不适合无脑硬灌坦克。".to_string());
    }
    if enemy.crowd_control >= 75 {
        threats.push("敌方控制链很足，被先手命中后容易连续吃技能。".to_string());
    }
    if enemy.scaling >= 75 {
        threats.push("敌方后期能力较强，资源节奏不能完全放给对面。".to_string());
    }
    if enemy.frontline >= 75 && my.magic_damage <= 40 {
        threats.push("敌方可以更放心堆护甲，我方需要用节奏和持续输出处理前排。".to_string());
    }

    threats
}

fn win_conditions_for(my: &CompositionDimensions, enemy: &CompositionDimensions) -> Vec<String> {
    let mut conditions = Vec::new();

    if my.engage >= 65 && my.crowd_control >= 65 {
        conditions.push("利用我方控制链先手，优先逼出敌方关键进场或保命技能。".to_string());
    }
    if my.frontline >= 65 {
        conditions.push("让我方前排先占河道和野区入口，后排保持输出距离打拉扯。".to_string());
    }
    if my.scaling >= 65 {
        conditions.push("稳住核心装备前少接无视野窄口团，成型后用反开和持续输出赢团。".to_string());
    }
    if my.physical_damage >= 65 && my.magic_damage <= 40 {
        conditions.push("围绕物理核心滚雪球，尽早建立资源差，降低敌方堆护甲后的压力。".to_string());
    }
    if enemy.engage >= 75 {
        conditions.push("团战站位要分散，留关键控制给敌方第一波进场。".to_string());
    }
    if conditions.is_empty() {
        conditions.push("先保证视野和兵线，再根据敌方露出的关键技能寻找小规模机会。".to_string());
    }

    conditions
}

fn suggestions_for(dimensions: &CompositionDimensions, draft_complete: bool) -> Vec<String> {
    let mut suggestions = Vec::new();

    if dimensions.frontline <= 35 {
        suggestions.push(if draft_complete {
            "阵容前排不足，实战里尽量避免无视野正面硬接。".to_string()
        } else {
            "后续选人优先补一个能站前排的英雄。".to_string()
        });
    }
    if dimensions.engage <= 35 {
        suggestions.push(if draft_complete {
            "阵容主动开团偏弱，实战里多靠视野、兵线和反打找机会。".to_string()
        } else {
            "后续选人可以考虑补主动开团或强先手。".to_string()
        });
    }
    if dimensions.magic_damage <= 40 {
        suggestions.push(if draft_complete {
            "阵容 AP 压力偏低，实战里要用节奏、破甲和持续输出处理敌方前排。".to_string()
        } else {
            "后续选人尽量补充 AP 伤害。".to_string()
        });
    }
    if dimensions.crowd_control <= 35 {
        suggestions.push(if draft_complete {
            "阵容稳定控制偏少，实战里不要轻易交掉唯一留人技能。".to_string()
        } else {
            "后续选人可以补稳定控制，提升抓机会能力。".to_string()
        });
    }

    suggestions
}

fn counterplay_suggestions_for(
    my: &CompositionDimensions,
    enemy: &CompositionDimensions,
) -> Vec<String> {
    let mut suggestions = Vec::new();

    if enemy.engage >= 75 && enemy.crowd_control >= 65 {
        suggestions.push("面对敌方强开，避免五人挤在狭窄入口，先用视野逼他们交开团。".to_string());
    }
    if enemy.frontline >= 75 && my.magic_damage <= 40 {
        suggestions.push("打厚前排时不要急着一波灌死坦克，优先保护物理核心持续输出。".to_string());
    }
    if enemy.scaling >= 75 && my.scaling >= 65 {
        suggestions.push("双方都有后期能力时，关键是小龙、先锋和大龙前的提前站位。".to_string());
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
        assert!(tags.contains_key(&111));
        assert!(tags.contains_key(&901));
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

    #[test]
    fn compares_both_teams_and_reports_enemy_threats() {
        let mut draft = DraftState::empty("ChampSelect".to_string());
        draft.my_team = vec![player(22), player(104), player(89), player(86), player(117)];
        draft.their_team = vec![player(111), player(54), player(901), player(90), player(56)];

        let analysis = analyze_draft(&draft);

        assert_eq!(analysis.confidence, AnalysisConfidence::High);
        assert!(analysis.enemy_dimensions.engage >= 75);
        assert!(
            analysis
                .enemy_threats
                .iter()
                .any(|threat| threat.contains("强先手"))
        );
        assert!(
            analysis
                .win_conditions
                .iter()
                .any(|condition| condition.contains("站位"))
        );
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
