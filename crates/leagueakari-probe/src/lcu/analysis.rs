use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::models::{DraftPlayer, DraftState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionAnalysis {
    pub confidence: AnalysisConfidence,
    pub dimensions: CompositionDimensions,
    pub enemy_dimensions: CompositionDimensions,
    pub data_notes: Vec<String>,
    pub champion_stats: Vec<ChampionStatSummary>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChampionStatSummary {
    pub champion_id: i64,
    pub champion_key: String,
    pub role: String,
    pub win_rate: f32,
    pub pick_rate: f32,
    pub ban_rate: f32,
    pub rank: u16,
}

const TAGS_JSON: &str = include_str!("../../data/champion-tags.v1.json");
const OPGG_STATS_JSON: &str = include_str!("../../data/opgg-champion-stats.sample.json");

#[derive(Debug, Clone, Default, Deserialize)]
struct ChampionTags {
    champion_id: i64,
    champion_key: String,
    #[serde(default)]
    roles: Vec<String>,
    #[serde(default)]
    archetypes: Vec<String>,
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

#[derive(Debug, Clone, Deserialize)]
struct OpggStatsSnapshot {
    patch: String,
    tier: String,
    region: String,
    queue: String,
    sample_count: Option<u64>,
    entries: Vec<OpggChampionStat>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpggChampionStat {
    rank: u16,
    champion_key: String,
    role: String,
    win_rate: f32,
    pick_rate: f32,
    ban_rate: f32,
}

pub fn analyze_draft(draft: &DraftState) -> CompositionAnalysis {
    let tag_db = champion_tags();
    let stat_db = opgg_stats();
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
    let champion_stats = champion_stats_for(&draft.my_team, &draft.their_team, &tag_db, &stat_db);
    let data_notes = data_notes_for(&stat_db, champion_stats.len());
    let confidence = confidence_for(
        picked_champions.len() + enemy_champions.len(),
        known_tags.len() + enemy_known_tags.len(),
    );
    let strengths = strengths_for(&dimensions);
    let mut risks = risks_for(&dimensions);
    risks.extend(matchup_risks_for(&dimensions, &enemy_dimensions));
    risks.extend(meta_risks_for(&champion_stats));
    let enemy_threats = enemy_threats_for(&dimensions, &enemy_dimensions);
    let win_conditions = win_conditions_for(&dimensions, &enemy_dimensions);
    let mut suggestions = suggestions_for(&dimensions, picked_champions.len() >= 5);
    suggestions.extend(counterplay_suggestions_for(&dimensions, &enemy_dimensions));
    suggestions.extend(archetype_suggestions_for(&known_tags, &enemy_known_tags));
    suggestions.extend(meta_suggestions_for(&champion_stats));

    CompositionAnalysis {
        confidence,
        dimensions,
        enemy_dimensions,
        data_notes,
        champion_stats,
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

fn champion_stats_for(
    my_players: &[DraftPlayer],
    enemy_players: &[DraftPlayer],
    tag_db: &HashMap<i64, ChampionTags>,
    stat_db: &HashMap<String, OpggChampionStat>,
) -> Vec<ChampionStatSummary> {
    my_players
        .iter()
        .chain(enemy_players.iter())
        .filter_map(|player| {
            let champion_id = player.champion_id?;
            let tags = tag_db.get(&champion_id)?;
            let stat = best_stat_for(tags, player.assigned_position.as_deref(), stat_db)?;
            Some(ChampionStatSummary {
                champion_id,
                champion_key: tags.champion_key.clone(),
                role: stat.role.clone(),
                win_rate: stat.win_rate,
                pick_rate: stat.pick_rate,
                ban_rate: stat.ban_rate,
                rank: stat.rank,
            })
        })
        .collect()
}

fn best_stat_for<'a>(
    tags: &ChampionTags,
    assigned_position: Option<&str>,
    stat_db: &'a HashMap<String, OpggChampionStat>,
) -> Option<&'a OpggChampionStat> {
    if let Some(stat) = assigned_position
        .into_iter()
        .flat_map(opgg_roles_for)
        .find_map(|role| stat_db.get(&stat_key(&tags.champion_key, role)))
    {
        return Some(stat);
    }

    tags.roles
        .iter()
        .flat_map(|role| opgg_roles_for(role))
        .find_map(|role| stat_db.get(&stat_key(&tags.champion_key, role)))
        .or_else(|| stat_db.get(&stat_key(&tags.champion_key, "overall")))
}

fn data_notes_for(
    stat_db: &HashMap<String, OpggChampionStat>,
    matched_champion_stats: usize,
) -> Vec<String> {
    let snapshot = opgg_snapshot();
    let mut notes = vec![format!(
        "OP.GG 快照：{} / {} / {} / {}，样本量 {}。",
        snapshot.patch,
        snapshot.region,
        snapshot.tier,
        snapshot.queue,
        snapshot
            .sample_count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "未知".to_string())
    )];

    if stat_db.is_empty() {
        notes.push("当前没有可用 OP.GG 统计缓存，分析仅使用本地英雄机制标签。".to_string());
    } else {
        notes.push(format!(
            "本局匹配到 {matched_champion_stats} 个英雄的 OP.GG 角色统计；统计只辅助解释，不直接承诺胜负。"
        ));
    }

    notes
}

fn meta_risks_for(champion_stats: &[ChampionStatSummary]) -> Vec<String> {
    let mut risks = Vec::new();

    let high_ban = champion_stats
        .iter()
        .filter(|stat| stat.ban_rate >= 15.0)
        .collect::<Vec<_>>();
    if !high_ban.is_empty() {
        let names = high_ban
            .iter()
            .map(|stat| stat.champion_key.as_str())
            .collect::<Vec<_>>()
            .join("、");
        risks.push(format!(
            "OP.GG 当前版本里 {names} 禁用率较高，对局里通常代表处理成本或心理压力更高。"
        ));
    }

    risks
}

fn meta_suggestions_for(champion_stats: &[ChampionStatSummary]) -> Vec<String> {
    let mut suggestions = Vec::new();

    let low_win_high_pick = champion_stats
        .iter()
        .filter(|stat| stat.win_rate < 49.5 && stat.pick_rate >= 10.0)
        .collect::<Vec<_>>();
    if !low_win_high_pick.is_empty() {
        let names = low_win_high_pick
            .iter()
            .map(|stat| stat.champion_key.as_str())
            .collect::<Vec<_>>()
            .join("、");
        suggestions.push(format!(
            "OP.GG 快照里 {names} 属于高登场但胜率偏低，实战更要关注熟练度和阵容配合。"
        ));
    }

    suggestions
}

fn archetype_suggestions_for(
    my_tags: &[&ChampionTags],
    enemy_tags: &[&ChampionTags],
) -> Vec<String> {
    let mut suggestions = Vec::new();

    if has_archetype(enemy_tags, "pick") && !has_archetype(my_tags, "peel") {
        suggestions
            .push("敌方抓单能力较强，我方保护标签不足，边线和野区入口要避免单人脸探。".to_string());
    }
    if has_archetype(my_tags, "scaling") && has_archetype(enemy_tags, "early_tempo") {
        suggestions
            .push("我方有成长点但敌方前期节奏更强，前两条小龙不必硬换全队节奏。".to_string());
    }
    if has_archetype(my_tags, "poke") && has_archetype(enemy_tags, "engage") {
        suggestions
            .push("我方有消耗能力时，团前先拉开距离压血线，不要让敌方直接强开满血团。".to_string());
    }

    suggestions
}

fn has_archetype(tags: &[&ChampionTags], archetype: &str) -> bool {
    tags.iter()
        .any(|tags| tags.archetypes.iter().any(|value| value == archetype))
}

fn champion_tags() -> HashMap<i64, ChampionTags> {
    serde_json::from_str::<Vec<ChampionTags>>(TAGS_JSON)
        .unwrap_or_default()
        .into_iter()
        .map(|tags| (tags.champion_id, tags))
        .collect()
}

fn opgg_stats() -> HashMap<String, OpggChampionStat> {
    opgg_snapshot()
        .entries
        .into_iter()
        .map(|entry| (stat_key(&entry.champion_key, &entry.role), entry))
        .collect()
}

fn opgg_snapshot() -> OpggStatsSnapshot {
    serde_json::from_str::<OpggStatsSnapshot>(OPGG_STATS_JSON).unwrap_or(OpggStatsSnapshot {
        patch: "unknown".to_string(),
        tier: "unknown".to_string(),
        region: "unknown".to_string(),
        queue: "unknown".to_string(),
        sample_count: None,
        entries: Vec::new(),
    })
}

fn stat_key(champion_key: &str, role: &str) -> String {
    format!("{champion_key}:{role}")
}

fn opgg_roles_for(role: &str) -> &'static [&'static str] {
    match role {
        "bottom" => &["adc"],
        "utility" => &["support"],
        "middle" => &["mid"],
        "top" => &["top"],
        "jungle" => &["jungle"],
        _ => &[],
    }
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
        let tags = champion_tags();

        assert!(tags.contains_key(&103));
        assert!(tags.contains_key(&22));
        assert!(tags.contains_key(&111));
        assert!(tags.contains_key(&901));
    }

    #[test]
    fn loads_opgg_stats_snapshot() {
        let stats = opgg_stats();

        assert!(stats.contains_key("ahri:mid"));
        assert!(stats.contains_key("ashe:adc"));
        assert!(stats.contains_key("nautilus:support"));
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
                .champion_stats
                .iter()
                .any(|stat| stat.champion_key == "ashe" && stat.role == "adc")
        );
        assert!(
            analysis
                .data_notes
                .iter()
                .any(|note| note.contains("OP.GG 快照"))
        );
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
