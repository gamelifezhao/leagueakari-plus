use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    champions::ChampionCatalog,
    client::LcuClient,
    models::{DraftPlayerIdentity, DraftState},
};

#[derive(Debug, Clone, Serialize)]
pub struct TeammatePerformance {
    pub cell_id: i64,
    pub display_name: String,
    pub champion_id: Option<i64>,
    pub assigned_position: Option<String>,
    pub team_type: Option<String>,
    pub games: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f32,
    pub avg_kills: f32,
    pub avg_deaths: f32,
    pub avg_assists: f32,
    pub kd_ratio: f32,
    pub tier: TeammateTier,
    pub tier_label: &'static str,
    pub recent_matches: Vec<TeammateRecentMatch>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TeammateRecentMatch {
    pub game_id: i64,
    pub queue_id: i64,
    pub queue_label: &'static str,
    pub result: &'static str,
    pub result_label: &'static str,
    pub ended_at_label: String,
    pub duration_seconds: i64,
    pub champion_id: i64,
    pub champion_name: String,
    pub champion_alias: Option<String>,
    pub position: String,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub items: Vec<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TeammateTier {
    TopHorse,
    Stable,
    LowHorse,
    Workhorse,
    DataLight,
}

#[derive(Default)]
pub struct TeammatePerformanceCache {
    stats_by_puuid: HashMap<String, CachedTeammateStats>,
}

#[derive(Debug, Clone)]
struct CachedTeammateStats {
    display_name: String,
    games: usize,
    wins: usize,
    losses: usize,
    win_rate: f32,
    avg_kills: f32,
    avg_deaths: f32,
    avg_assists: f32,
    kd_ratio: f32,
    tier: TeammateTier,
    recent_matches: Vec<TeammateRecentMatch>,
}

#[derive(Debug, Deserialize)]
struct MatchHistory {
    games: MatchHistoryGames,
}

#[derive(Debug, Deserialize)]
struct MatchHistoryGames {
    games: Vec<MatchHistoryGame>,
}

#[derive(Debug, Deserialize)]
struct MatchHistoryGame {
    #[serde(default, rename = "gameId")]
    game_id: i64,
    #[serde(default, rename = "queueId")]
    queue_id: i64,
    #[serde(default, rename = "gameCreation")]
    game_creation: i64,
    #[serde(default, rename = "gameCreationDate")]
    game_creation_date: String,
    #[serde(default, rename = "gameDuration")]
    game_duration: i64,
    #[serde(rename = "participantIdentities")]
    participant_identities: Vec<ParticipantIdentity>,
    participants: Vec<Participant>,
}

#[derive(Debug, Deserialize)]
struct ParticipantIdentity {
    #[serde(rename = "participantId")]
    participant_id: i64,
    player: MatchHistoryPlayer,
}

#[derive(Debug, Deserialize)]
struct MatchHistoryPlayer {
    puuid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Participant {
    #[serde(rename = "participantId")]
    participant_id: i64,
    #[serde(default, rename = "championId")]
    champion_id: i64,
    #[serde(default)]
    timeline: ParticipantTimeline,
    stats: ParticipantStats,
}

#[derive(Debug, Default, Deserialize)]
struct ParticipantTimeline {
    #[serde(default)]
    lane: String,
    #[serde(default)]
    role: String,
}

#[derive(Debug, Deserialize)]
struct ParticipantStats {
    kills: u32,
    deaths: u32,
    assists: u32,
    #[serde(default)]
    win: Value,
    #[serde(default)]
    item0: u32,
    #[serde(default)]
    item1: u32,
    #[serde(default)]
    item2: u32,
    #[serde(default)]
    item3: u32,
    #[serde(default)]
    item4: u32,
    #[serde(default)]
    item5: u32,
    #[serde(default)]
    item6: u32,
}

pub async fn analyze_teammates(
    client: &LcuClient,
    draft_state: &DraftState,
    current_summoner: Option<&Value>,
    champion_catalog: &ChampionCatalog,
    cache: &mut TeammatePerformanceCache,
) -> Vec<TeammatePerformance> {
    let current_puuid = current_summoner.and_then(|summoner| {
        summoner
            .get("puuid")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
    });
    let teammates = teammate_identities(client, draft_state, current_puuid.as_deref()).await;
    let mut performance = Vec::new();

    for teammate in teammates {
        let Some(puuid) = teammate.puuid.as_deref() else {
            performance.push(unavailable_performance(teammate));
            continue;
        };
        if current_puuid.as_deref() == Some(puuid) {
            continue;
        }

        let stats = if let Some(stats) = cache.stats_by_puuid.get(puuid) {
            stats.clone()
        } else {
            let stats = fetch_teammate_stats(
                client,
                puuid,
                teammate.display_name.as_deref(),
                champion_catalog,
            )
            .await;
            cache
                .stats_by_puuid
                .insert(puuid.to_string(), stats.clone());
            stats
        };

        performance.push(TeammatePerformance {
            cell_id: teammate.cell_id,
            display_name: stats.display_name,
            champion_id: teammate.champion_id,
            assigned_position: teammate.assigned_position,
            team_type: teammate.team_type,
            games: stats.games,
            wins: stats.wins,
            losses: stats.losses,
            win_rate: stats.win_rate,
            avg_kills: stats.avg_kills,
            avg_deaths: stats.avg_deaths,
            avg_assists: stats.avg_assists,
            kd_ratio: stats.kd_ratio,
            tier: stats.tier,
            tier_label: tier_label(stats.tier),
            recent_matches: stats.recent_matches,
        });
    }

    performance
}

async fn teammate_identities(
    client: &LcuClient,
    draft_state: &DraftState,
    current_puuid: Option<&str>,
) -> Vec<DraftPlayerIdentity> {
    if draft_state.gameflow == "ChampSelect" {
        return teammates_from_champ_select(client, draft_state, current_puuid).await;
    }

    let draft_teammates = teammates_from_draft_state(draft_state, current_puuid);
    if draft_teammates
        .iter()
        .any(|teammate| teammate.puuid.is_some())
    {
        return draft_teammates;
    }

    if let Some(teammates) = teammates_from_gameflow_session(client, current_puuid).await {
        return teammates;
    }

    teammates_from_champ_select(client, draft_state, current_puuid).await
}

fn teammates_from_draft_state(
    draft_state: &DraftState,
    current_puuid: Option<&str>,
) -> Vec<DraftPlayerIdentity> {
    draft_state
        .my_team
        .iter()
        .map(|player| (player, "ally"))
        .chain(
            draft_state
                .their_team
                .iter()
                .map(|player| (player, "enemy")),
        )
        .filter(|player| {
            Some(player.0.cell_id) != draft_state.local_player_cell_id
                && player.0.puuid.as_deref() != current_puuid
        })
        .map(|(player, team_type)| DraftPlayerIdentity {
            cell_id: player.cell_id,
            champion_id: player.champion_id,
            assigned_position: player.assigned_position.clone(),
            puuid: player.puuid.clone(),
            display_name: player.display_name.clone(),
            team_type: Some(team_type.to_string()),
        })
        .collect()
}

async fn teammates_from_gameflow_session(
    client: &LcuClient,
    current_puuid: Option<&str>,
) -> Option<Vec<DraftPlayerIdentity>> {
    let session = client
        .get_json::<Value>("/lol-gameflow/v1/session")
        .await
        .ok()?;
    let game_data = session.get("gameData")?;
    let team_one = game_data.get("teamOne").and_then(Value::as_array)?;
    let team_two = game_data.get("teamTwo").and_then(Value::as_array)?;
    let local_team = if let Some(current_puuid) = current_puuid {
        if team_one
            .iter()
            .any(|player| player_puuid(player) == Some(current_puuid))
        {
            team_one
        } else if team_two
            .iter()
            .any(|player| player_puuid(player) == Some(current_puuid))
        {
            team_two
        } else {
            team_one
        }
    } else {
        team_one
    };

    let mut teammates = Vec::new();
    for (index, player) in local_team.iter().enumerate() {
        let puuid = player_puuid(player).map(ToOwned::to_owned);
        if puuid.as_deref() == current_puuid {
            continue;
        }
        teammates.push(DraftPlayerIdentity {
            cell_id: index as i64,
            champion_id: positive_i64(player.get("championId")),
            assigned_position: player
                .get("selectedPosition")
                .and_then(Value::as_str)
                .and_then(position_label),
            puuid,
            display_name: display_name_from_summoner_value(player),
            team_type: Some("ally".to_string()),
        });
    }

    Some(teammates)
}

async fn teammates_from_champ_select(
    client: &LcuClient,
    draft_state: &DraftState,
    current_puuid: Option<&str>,
) -> Vec<DraftPlayerIdentity> {
    let mut teammates = Vec::new();
    for player in &draft_state.my_team {
        if Some(player.cell_id) == draft_state.local_player_cell_id {
            continue;
        }

        let Some(summoner_id) = player.summoner_id else {
            teammates.push(DraftPlayerIdentity {
                cell_id: player.cell_id,
                champion_id: player.champion_id,
                assigned_position: player.assigned_position.clone(),
                puuid: None,
                display_name: None,
                team_type: Some("ally".to_string()),
            });
            continue;
        };

        let summoner = client
            .get_json::<Value>(&format!("/lol-summoner/v1/summoners/{summoner_id}"))
            .await
            .ok();
        let puuid = summoner
            .as_ref()
            .and_then(|value| value.get("puuid"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        if puuid.as_deref() == current_puuid {
            continue;
        }

        teammates.push(DraftPlayerIdentity {
            cell_id: player.cell_id,
            champion_id: player.champion_id,
            assigned_position: player.assigned_position.clone(),
            puuid,
            display_name: summoner.as_ref().and_then(display_name_from_summoner_value),
            team_type: Some("ally".to_string()),
        });
    }

    teammates
}

async fn fetch_teammate_stats(
    client: &LcuClient,
    puuid: &str,
    fallback_name: Option<&str>,
    champion_catalog: &ChampionCatalog,
) -> CachedTeammateStats {
    let display_name = resolve_display_name(client, puuid)
        .await
        .or_else(|| fallback_name.map(ToOwned::to_owned))
        .unwrap_or_else(|| "队友".to_string());

    let history = client
        .get_json::<MatchHistory>(&format!(
            "/lol-match-history/v1/products/lol/{puuid}/matches?begIndex=0&endIndex=19"
        ))
        .await;
    let Ok(history) = history else {
        return CachedTeammateStats {
            display_name,
            games: 0,
            wins: 0,
            losses: 0,
            win_rate: 0.0,
            avg_kills: 0.0,
            avg_deaths: 0.0,
            avg_assists: 0.0,
            kd_ratio: 0.0,
            tier: TeammateTier::DataLight,
            recent_matches: Vec::new(),
        };
    };

    summarize_match_history(display_name, puuid, history, champion_catalog)
}

async fn resolve_display_name(client: &LcuClient, puuid: &str) -> Option<String> {
    client
        .get_json::<Value>(&format!("/lol-summoner/v2/summoners/puuid/{puuid}"))
        .await
        .ok()
        .as_ref()
        .and_then(display_name_from_summoner_value)
}

fn summarize_match_history(
    display_name: String,
    puuid: &str,
    history: MatchHistory,
    champion_catalog: &ChampionCatalog,
) -> CachedTeammateStats {
    let mut games = 0_u32;
    let mut kills = 0_u32;
    let mut deaths = 0_u32;
    let mut assists = 0_u32;
    let mut wins = 0_u32;
    let mut recent_matches = Vec::new();

    for game in history.games.games.into_iter().take(20) {
        let Some(participant_id) = participant_id_for_puuid(&game, puuid) else {
            continue;
        };
        let Some(participant) = game
            .participants
            .iter()
            .find(|participant| participant.participant_id == participant_id)
        else {
            continue;
        };

        games += 1;
        if win_field(&participant.stats.win) {
            wins += 1;
        }
        kills += participant.stats.kills;
        deaths += participant.stats.deaths;
        assists += participant.stats.assists;
        if recent_matches.len() < 8 {
            recent_matches.push(teammate_recent_match(&game, participant, champion_catalog));
        }
    }

    if games == 0 {
        return CachedTeammateStats {
            display_name,
            games: 0,
            wins: 0,
            losses: 0,
            win_rate: 0.0,
            avg_kills: 0.0,
            avg_deaths: 0.0,
            avg_assists: 0.0,
            kd_ratio: 0.0,
            tier: TeammateTier::DataLight,
            recent_matches,
        };
    }

    let games_f = games as f32;
    let losses = games.saturating_sub(wins);
    let kd_ratio = kills as f32 / (deaths.max(1) as f32);
    let avg_kills = kills as f32 / games_f;
    let avg_deaths = deaths as f32 / games_f;
    let avg_assists = assists as f32 / games_f;
    let kda_ratio = (kills + assists) as f32 / (deaths.max(1) as f32);
    let tier = tier_for(games as usize, kd_ratio, kda_ratio, avg_deaths);

    CachedTeammateStats {
        display_name,
        games: games as usize,
        wins: wins as usize,
        losses: losses as usize,
        win_rate: (wins as f32 / games_f) * 100.0,
        avg_kills,
        avg_deaths,
        avg_assists,
        kd_ratio,
        tier,
        recent_matches,
    }
}

fn teammate_recent_match(
    game: &MatchHistoryGame,
    participant: &Participant,
    champion_catalog: &ChampionCatalog,
) -> TeammateRecentMatch {
    let won = win_field(&participant.stats.win);
    let champion = champion_catalog.get(participant.champion_id);
    TeammateRecentMatch {
        game_id: game.game_id,
        queue_id: game.queue_id,
        queue_label: queue_label(game.queue_id),
        result: if won { "win" } else { "loss" },
        result_label: if won { "胜利" } else { "失败" },
        ended_at_label: ended_at_label(game),
        duration_seconds: game.game_duration,
        champion_id: participant.champion_id,
        champion_name: champion
            .map(|champion| champion.name.clone())
            .unwrap_or_else(|| format!("英雄 {}", participant.champion_id)),
        champion_alias: champion.map(|champion| champion.alias.clone()),
        position: position_from_timeline(&participant.timeline),
        kills: participant.stats.kills,
        deaths: participant.stats.deaths,
        assists: participant.stats.assists,
        items: item_ids(&participant.stats),
    }
}

fn participant_id_for_puuid(game: &MatchHistoryGame, puuid: &str) -> Option<i64> {
    game.participant_identities
        .iter()
        .find(|identity| identity.player.puuid.as_deref() == Some(puuid))
        .map(|identity| identity.participant_id)
}

fn unavailable_performance(teammate: DraftPlayerIdentity) -> TeammatePerformance {
    TeammatePerformance {
        cell_id: teammate.cell_id,
        display_name: teammate.display_name.unwrap_or_else(|| "队友".to_string()),
        champion_id: teammate.champion_id,
        assigned_position: teammate.assigned_position,
        team_type: teammate.team_type,
        games: 0,
        wins: 0,
        losses: 0,
        win_rate: 0.0,
        avg_kills: 0.0,
        avg_deaths: 0.0,
        avg_assists: 0.0,
        kd_ratio: 0.0,
        tier: TeammateTier::DataLight,
        tier_label: tier_label(TeammateTier::DataLight),
        recent_matches: Vec::new(),
    }
}

fn tier_for(games: usize, kd_ratio: f32, kda_ratio: f32, avg_deaths: f32) -> TeammateTier {
    if games < 5 {
        TeammateTier::DataLight
    } else if kd_ratio >= 1.55 || (kd_ratio >= 1.25 && kda_ratio >= 2.6 && avg_deaths <= 5.8) {
        TeammateTier::TopHorse
    } else if kd_ratio < 0.75 || (kd_ratio < 0.95 && avg_deaths >= 7.2) {
        TeammateTier::Workhorse
    } else if kd_ratio < 0.95 || avg_deaths >= 7.0 {
        TeammateTier::LowHorse
    } else {
        TeammateTier::Stable
    }
}

fn tier_label(tier: TeammateTier) -> &'static str {
    match tier {
        TeammateTier::TopHorse => "上等马",
        TeammateTier::Stable => "普通",
        TeammateTier::LowHorse => "下等马",
        TeammateTier::Workhorse => "牛马",
        TeammateTier::DataLight => "数据少",
    }
}

fn player_puuid(value: &Value) -> Option<&str> {
    value.get("puuid").and_then(Value::as_str)
}

fn positive_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(Value::as_i64).filter(|id| *id > 0)
}

fn display_name_from_summoner_value(value: &Value) -> Option<String> {
    let game_name = value.get("gameName").and_then(Value::as_str);
    let tag_line = value.get("tagLine").and_then(Value::as_str);
    match (game_name, tag_line) {
        (Some(game_name), Some(tag_line)) if !game_name.is_empty() && !tag_line.is_empty() => {
            Some(format!("{game_name}#{tag_line}"))
        }
        (Some(game_name), _) if !game_name.is_empty() => Some(game_name.to_string()),
        _ => value
            .get("summonerName")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .map(ToOwned::to_owned),
    }
}

fn position_label(position: &str) -> Option<String> {
    match position {
        "TOP" => Some("top".to_string()),
        "JUNGLE" => Some("jungle".to_string()),
        "MIDDLE" => Some("middle".to_string()),
        "BOTTOM" => Some("bottom".to_string()),
        "UTILITY" => Some("utility".to_string()),
        _ => None,
    }
}

fn position_from_timeline(timeline: &ParticipantTimeline) -> String {
    match (timeline.lane.as_str(), timeline.role.as_str()) {
        ("TOP", _) => "top".to_string(),
        ("JUNGLE", _) => "jungle".to_string(),
        ("MIDDLE", _) => "middle".to_string(),
        ("BOTTOM", "DUO_CARRY") => "bottom".to_string(),
        ("BOTTOM", "DUO_SUPPORT") => "utility".to_string(),
        ("BOTTOM", _) => "bottom".to_string(),
        _ => "unknown".to_string(),
    }
}

fn queue_label(queue_id: i64) -> &'static str {
    match queue_id {
        420 => "单双排",
        430 => "匹配",
        440 => "灵活排位",
        450 => "极地大乱斗",
        490 => "快速匹配",
        700 => "冠军杯赛",
        900 => "无限火力",
        1700 => "斗魂竞技场",
        _ => "召唤师峡谷",
    }
}

fn ended_at_label(game: &MatchHistoryGame) -> String {
    if game.game_creation <= 0 {
        if !game.game_creation_date.is_empty() {
            return game.game_creation_date.clone();
        }
        return "时间未知".to_string();
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(game.game_creation);
    let elapsed_ms = now_ms.saturating_sub(game.game_creation);
    let elapsed_seconds = elapsed_ms / 1000;
    if elapsed_seconds < 3600 {
        let minutes = (elapsed_seconds / 60).max(1);
        format!("{minutes} 分钟前")
    } else if elapsed_seconds < 86_400 {
        format!("{} 小时前", elapsed_seconds / 3600)
    } else {
        format!("{} 天前", elapsed_seconds / 86_400)
    }
}

fn item_ids(stats: &ParticipantStats) -> Vec<i64> {
    [
        stats.item0,
        stats.item1,
        stats.item2,
        stats.item3,
        stats.item4,
        stats.item5,
        stats.item6,
    ]
    .into_iter()
    .filter(|item_id| *item_id > 0)
    .map(i64::from)
    .collect()
}

fn win_field(value: &Value) -> bool {
    if let Some(win) = value.as_bool() {
        return win;
    }
    value
        .as_str()
        .is_some_and(|value| matches!(value, "Win" | "win" | "true"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn tiers_teammates_by_recent_kd() {
        assert_eq!(tier_for(20, 1.8, 3.0, 4.5), TeammateTier::TopHorse);
        assert_eq!(tier_for(20, 1.08, 1.9, 6.2), TeammateTier::Stable);
        assert_eq!(tier_for(20, 1.0, 1.6, 7.4), TeammateTier::LowHorse);
        assert_eq!(tier_for(20, 0.5, 1.1, 7.8), TeammateTier::Workhorse);
        assert_eq!(tier_for(3, 9.0, 12.0, 1.0), TeammateTier::DataLight);
    }

    #[test]
    fn summarizes_match_history_for_puuid() {
        let catalog = super::super::champions::ChampionCatalog::from_lcu_summary(&json!([
            { "id": 103, "alias": "Ahri", "name": "九尾妖狐" }
        ]));
        let history = serde_json::from_value::<MatchHistory>(json!({
            "games": {
                "games": [
                    {
                        "gameId": 1,
                        "queueId": 420,
                        "gameCreation": 1760000000000i64,
                        "gameDuration": 1800,
                        "participantIdentities": [
                            { "participantId": 1, "player": { "puuid": "target" } }
                        ],
                        "participants": [
                            { "participantId": 1, "championId": 103, "timeline": { "lane": "MIDDLE", "role": "SOLO" }, "stats": { "kills": 10, "deaths": 5, "assists": 2, "win": true, "item0": 1056 } }
                        ]
                    },
                    {
                        "gameId": 2,
                        "queueId": 440,
                        "gameCreation": 1760000000000i64,
                        "gameDuration": 1500,
                        "participantIdentities": [
                            { "participantId": 3, "player": { "puuid": "target" } }
                        ],
                        "participants": [
                            { "participantId": 3, "championId": 103, "timeline": { "lane": "MIDDLE", "role": "SOLO" }, "stats": { "kills": 4, "deaths": 3, "assists": 10, "win": false, "item0": 3089 } }
                        ]
                    }
                ]
            }
        }))
        .unwrap();

        let stats = summarize_match_history("队友".to_string(), "target", history, &catalog);

        assert_eq!(stats.games, 2);
        assert_eq!(stats.wins, 1);
        assert_eq!(stats.losses, 1);
        assert_eq!(stats.avg_kills, 7.0);
        assert_eq!(stats.avg_deaths, 4.0);
        assert_eq!(stats.avg_assists, 6.0);
        assert!((stats.kd_ratio - 1.75).abs() < f32::EPSILON);
        assert_eq!(stats.recent_matches.len(), 2);
        assert_eq!(
            stats.recent_matches[0].champion_alias.as_deref(),
            Some("Ahri")
        );
    }
}
