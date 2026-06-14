use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    client::LcuClient,
    models::{DraftPlayerIdentity, DraftState},
};

#[derive(Debug, Clone, Serialize)]
pub struct TeammatePerformance {
    pub cell_id: i64,
    pub display_name: String,
    pub champion_id: Option<i64>,
    pub assigned_position: Option<String>,
    pub games: usize,
    pub avg_kills: f32,
    pub avg_deaths: f32,
    pub avg_assists: f32,
    pub kd_ratio: f32,
    pub tier: TeammateTier,
    pub tier_label: &'static str,
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
    avg_kills: f32,
    avg_deaths: f32,
    avg_assists: f32,
    kd_ratio: f32,
    tier: TeammateTier,
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
    stats: ParticipantStats,
}

#[derive(Debug, Deserialize)]
struct ParticipantStats {
    kills: u32,
    deaths: u32,
    assists: u32,
}

pub async fn analyze_teammates(
    client: &LcuClient,
    draft_state: &DraftState,
    current_summoner: Option<&Value>,
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

    for teammate in teammates.into_iter().take(4) {
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
            let stats = fetch_teammate_stats(client, puuid, teammate.display_name.as_deref()).await;
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
            games: stats.games,
            avg_kills: stats.avg_kills,
            avg_deaths: stats.avg_deaths,
            avg_assists: stats.avg_assists,
            kd_ratio: stats.kd_ratio,
            tier: stats.tier,
            tier_label: tier_label(stats.tier),
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

    if let Some(teammates) = teammates_from_gameflow_session(client, current_puuid).await {
        return teammates;
    }

    teammates_from_champ_select(client, draft_state, current_puuid).await
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
        });
    }

    teammates
}

async fn fetch_teammate_stats(
    client: &LcuClient,
    puuid: &str,
    fallback_name: Option<&str>,
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
            avg_kills: 0.0,
            avg_deaths: 0.0,
            avg_assists: 0.0,
            kd_ratio: 0.0,
            tier: TeammateTier::DataLight,
        };
    };

    summarize_match_history(display_name, puuid, history)
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
) -> CachedTeammateStats {
    let mut games = 0_u32;
    let mut kills = 0_u32;
    let mut deaths = 0_u32;
    let mut assists = 0_u32;

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
        kills += participant.stats.kills;
        deaths += participant.stats.deaths;
        assists += participant.stats.assists;
    }

    if games == 0 {
        return CachedTeammateStats {
            display_name,
            games: 0,
            avg_kills: 0.0,
            avg_deaths: 0.0,
            avg_assists: 0.0,
            kd_ratio: 0.0,
            tier: TeammateTier::DataLight,
        };
    }

    let games_f = games as f32;
    let kd_ratio = kills as f32 / (deaths.max(1) as f32);
    let avg_kills = kills as f32 / games_f;
    let avg_deaths = deaths as f32 / games_f;
    let avg_assists = assists as f32 / games_f;
    let kda_ratio = (kills + assists) as f32 / (deaths.max(1) as f32);
    let tier = tier_for(games as usize, kd_ratio, kda_ratio, avg_deaths);

    CachedTeammateStats {
        display_name,
        games: games as usize,
        avg_kills,
        avg_deaths,
        avg_assists,
        kd_ratio,
        tier,
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
        games: 0,
        avg_kills: 0.0,
        avg_deaths: 0.0,
        avg_assists: 0.0,
        kd_ratio: 0.0,
        tier: TeammateTier::DataLight,
        tier_label: tier_label(TeammateTier::DataLight),
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
        let history = serde_json::from_value::<MatchHistory>(json!({
            "games": {
                "games": [
                    {
                        "participantIdentities": [
                            { "participantId": 1, "player": { "puuid": "target" } }
                        ],
                        "participants": [
                            { "participantId": 1, "stats": { "kills": 10, "deaths": 5, "assists": 2 } }
                        ]
                    },
                    {
                        "participantIdentities": [
                            { "participantId": 3, "player": { "puuid": "target" } }
                        ],
                        "participants": [
                            { "participantId": 3, "stats": { "kills": 4, "deaths": 3, "assists": 10 } }
                        ]
                    }
                ]
            }
        }))
        .unwrap();

        let stats = summarize_match_history("队友".to_string(), "target", history);

        assert_eq!(stats.games, 2);
        assert_eq!(stats.avg_kills, 7.0);
        assert_eq!(stats.avg_deaths, 4.0);
        assert_eq!(stats.avg_assists, 6.0);
        assert!((stats.kd_ratio - 1.75).abs() < f32::EPSILON);
    }
}
