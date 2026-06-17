use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

use super::{
    champions::ChampionCatalog,
    client::{LcuClient, LcuClientError},
};

const MATCH_LIMIT: usize = 20;

#[derive(Debug, Error)]
pub enum MatchHistoryError {
    #[error("current summoner is unavailable, cannot request match history")]
    MissingSummoner,
    #[error("LCU match history request failed: {0}")]
    Lcu(#[from] LcuClientError),
    #[error("LCU match history response did not include games")]
    MissingGames,
}

#[derive(Debug, Serialize)]
pub struct RecentMatches {
    pub player: MatchHistoryPlayer,
    pub summary: MatchHistorySummary,
    pub matches: Vec<RecentMatch>,
}

#[derive(Debug, Serialize)]
pub struct MatchHistoryPlayer {
    pub display_name: String,
    pub puuid_hidden: bool,
}

#[derive(Debug, Serialize)]
pub struct MatchHistorySummary {
    pub total_games: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f64,
    pub avg_kda: f64,
    pub avg_kill_participation: f64,
    pub avg_damage_share: f64,
    pub favorite_champions: Vec<FavoriteChampion>,
}

#[derive(Debug, Serialize)]
pub struct FavoriteChampion {
    pub champion_id: i64,
    pub champion_name: String,
    pub champion_alias: Option<String>,
    pub games: usize,
    pub wins: usize,
}

#[derive(Debug, Serialize)]
pub struct RecentMatch {
    pub game_id: i64,
    pub queue_id: i64,
    pub queue_label: String,
    pub game_mode: String,
    pub game_version: String,
    pub result: String,
    pub result_label: String,
    pub ended_at_label: String,
    pub duration_seconds: i64,
    pub champion_id: i64,
    pub champion_name: String,
    pub champion_alias: Option<String>,
    pub position: String,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub kda: f64,
    pub kill_participation: f64,
    pub damage_share: f64,
    pub total_damage: i64,
    pub cs: i64,
    pub vision_score: i64,
    pub gold: i64,
    pub items: Vec<i64>,
    pub tags: Vec<String>,
    pub teams: Vec<Vec<MatchParticipant>>,
    pub timeline: Option<MatchTimeline>,
}

#[derive(Debug, Serialize)]
pub struct MatchParticipant {
    pub participant_id: i64,
    pub team_id: i64,
    pub display_name: String,
    pub champion_id: i64,
    pub champion_name: String,
    pub champion_alias: Option<String>,
    pub is_current_player: bool,
    pub position: String,
    pub level: i64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub kda: f64,
    pub kill_participation: f64,
    pub total_damage: i64,
    pub damage_taken: i64,
    pub cs: i64,
    pub cs_per_minute: f64,
    pub gold: i64,
    pub gold_per_minute: f64,
    pub vision_score: i64,
    pub spell_ids: Vec<i64>,
    pub items: Vec<i64>,
    pub rune_style_ids: Vec<i64>,
    pub perk_ids: Vec<i64>,
}

#[derive(Debug, Serialize)]
pub struct MatchTimeline {
    pub events: Vec<MatchTimelineEvent>,
    pub gold_series: Vec<MatchGoldFrame>,
}

#[derive(Debug, Serialize)]
pub struct MatchTimelineEvent {
    pub timestamp: i64,
    pub event_type: String,
    pub participant_id: Option<i64>,
    pub killer_id: Option<i64>,
    pub victim_id: Option<i64>,
    pub assisting_participant_ids: Vec<i64>,
    pub item_id: Option<i64>,
    pub skill_slot: Option<i64>,
    pub team_id: Option<i64>,
    pub monster_type: String,
    pub monster_sub_type: String,
    pub building_type: String,
    pub lane_type: String,
    pub tower_type: String,
}

#[derive(Debug, Serialize)]
pub struct MatchGoldFrame {
    pub timestamp: i64,
    pub blue_gold: i64,
    pub red_gold: i64,
}

#[derive(Debug, Clone)]
struct PlayerIdentity {
    puuid: Option<String>,
    display_name: String,
}

pub async fn fetch_recent_matches(
    client: &LcuClient,
    summoner: Option<&Value>,
    champion_catalog: &ChampionCatalog,
) -> Result<RecentMatches, MatchHistoryError> {
    let summoner = summoner.ok_or(MatchHistoryError::MissingSummoner)?;
    let current_puuid = summoner.get("puuid").and_then(Value::as_str);
    let path = current_puuid
        .map(|puuid| {
            format!(
                "/lol-match-history/v1/products/lol/{puuid}/matches?begIndex=0&endIndex={MATCH_LIMIT}"
            )
        })
        .unwrap_or_else(|| {
            format!(
                "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex={MATCH_LIMIT}"
            )
        });

    let value = match client.get_json::<Value>(&path).await {
        Ok(value) => value,
        Err(first_error) if current_puuid.is_some() => {
            tracing::debug!("puuid match-history request failed, falling back: {first_error}");
            client
                .get_json::<Value>(&format!(
                    "/lol-match-history/v1/products/lol/current-summoner/matches?begIndex=0&endIndex={MATCH_LIMIT}"
                ))
                .await?
        }
        Err(error) => return Err(error.into()),
    };

    let hydrated_value = hydrate_match_history(client, &value).await;
    summarize_match_history(&hydrated_value, summoner, champion_catalog)
}

async fn hydrate_match_history(client: &LcuClient, value: &Value) -> Value {
    let Some(games) = games_array(value) else {
        return value.clone();
    };
    let mut hydrated_games = Vec::new();

    for game in games.iter().take(MATCH_LIMIT) {
        let game_id = i64_field(game, "gameId");
        let mut game_object = if should_request_game_detail(game) {
            match client
                .get_json::<Value>(&format!("/lol-match-history/v1/games/{game_id}"))
                .await
            {
                Ok(detail) => detail_game_object(detail),
                Err(error) => {
                    tracing::debug!("match detail request failed for {game_id}: {error}");
                    game.clone()
                }
            }
        } else {
            game.clone()
        };

        if let Some(timeline) = fetch_game_timeline(client, game_id).await {
            if let Some(game_map) = game_object.as_object_mut() {
                game_map.insert("timeline".to_string(), timeline);
            }
        }

        hydrated_games.push(game_object);
    }

    let mut hydrated = value.clone();
    if let Some(games_value) = hydrated.pointer_mut("/games/games") {
        *games_value = Value::Array(hydrated_games);
    } else if let Some(games_value) = hydrated.get_mut("games") {
        *games_value = Value::Array(hydrated_games);
    }
    hydrated
}

fn should_request_game_detail(game: &Value) -> bool {
    let participant_count = game
        .get("participants")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let identity_count = game
        .get("participantIdentities")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let missing_stats = game
        .get("participants")
        .and_then(Value::as_array)
        .is_some_and(|participants| participants.iter().any(participant_missing_core_stats));
    i64_field(game, "gameId") > 0 && (participant_count < 10 || identity_count < 10 || missing_stats)
}

fn participant_missing_core_stats(participant: &Value) -> bool {
    let Some(stats) = participant.get("stats").filter(|value| value.is_object()) else {
        return true;
    };

    ["kills", "deaths", "assists", "totalDamageDealtToChampions"]
        .into_iter()
        .any(|key| stats.get(key).and_then(Value::as_i64).is_none())
}

fn detail_game_object(detail: Value) -> Value {
    detail
        .get("game")
        .filter(|game| game.is_object())
        .cloned()
        .unwrap_or(detail)
}

async fn fetch_game_timeline(client: &LcuClient, game_id: i64) -> Option<Value> {
    if game_id <= 0 {
        return None;
    }

    let timeline = client
        .get_json::<Value>(&format!("/lol-match-history/v1/game-timelines/{game_id}"))
        .await
        .ok()?;
    timeline.get("frames").and_then(Value::as_array)?;
    Some(timeline)
}

fn summarize_match_history(
    value: &Value,
    summoner: &Value,
    champion_catalog: &ChampionCatalog,
) -> Result<RecentMatches, MatchHistoryError> {
    let games = games_array(value).ok_or(MatchHistoryError::MissingGames)?;
    let current_puuid = summoner.get("puuid").and_then(Value::as_str);
    let current_display = summoner_display_name(summoner);
    let mut matches = Vec::new();

    for game in games.iter().take(MATCH_LIMIT) {
        if let Some(summary) =
            summarize_game(game, current_puuid, &current_display, champion_catalog)
        {
            matches.push(summary);
        }
    }

    Ok(RecentMatches {
        player: MatchHistoryPlayer {
            display_name: current_display,
            puuid_hidden: true,
        },
        summary: summarize_matches(&matches),
        matches,
    })
}

fn games_array(value: &Value) -> Option<&Vec<Value>> {
    value
        .pointer("/games/games")
        .and_then(Value::as_array)
        .or_else(|| value.get("games").and_then(Value::as_array))
}

fn summarize_game(
    game: &Value,
    current_puuid: Option<&str>,
    current_display: &str,
    champion_catalog: &ChampionCatalog,
) -> Option<RecentMatch> {
    let participants = game.get("participants")?.as_array()?;
    let identities = player_identities(game);
    let current_participant = participants.iter().find(|participant| {
        let participant_id = i64_field(participant, "participantId");
        identities
            .get(&participant_id)
            .is_some_and(|identity| is_current_identity(identity, current_puuid, current_display))
    })?;
    let stats = current_participant.get("stats")?;
    let team_id = i64_field(current_participant, "teamId");
    let team_participants = participants
        .iter()
        .filter(|participant| i64_field(participant, "teamId") == team_id)
        .collect::<Vec<_>>();
    let team_kills = team_participants
        .iter()
        .map(|participant| i64_field(participant.get("stats").unwrap_or(&Value::Null), "kills"))
        .sum::<i64>();
    let team_damage = team_participants
        .iter()
        .map(|participant| {
            i64_field(
                participant.get("stats").unwrap_or(&Value::Null),
                "totalDamageDealtToChampions",
            )
        })
        .sum::<i64>();

    let kills = i64_field(stats, "kills");
    let deaths = i64_field(stats, "deaths");
    let assists = i64_field(stats, "assists");
    let total_damage = i64_field(stats, "totalDamageDealtToChampions");
    let has_team_context = team_participants.len() > 1;
    let kill_participation = if has_team_context {
        percent(kills + assists, team_kills)
    } else {
        0.0
    };
    let damage_share = if has_team_context {
        percent(total_damage, team_damage)
    } else {
        0.0
    };
    let champion_id = i64_field(current_participant, "championId");
    let champion = champion_metadata(champion_catalog, champion_id);
    let result = if win_field(stats) { "win" } else { "loss" };

    Some(RecentMatch {
        game_id: i64_field(game, "gameId"),
        queue_id: i64_field(game, "queueId"),
        queue_label: queue_label(i64_field(game, "queueId")).to_string(),
        game_mode: string_field(game, "gameMode", "UNKNOWN"),
        game_version: string_field(game, "gameVersion", ""),
        result: result.to_string(),
        result_label: if result == "win" { "胜利" } else { "失败" }.to_string(),
        ended_at_label: ended_at_label(game),
        duration_seconds: i64_field(game, "gameDuration"),
        champion_id,
        champion_name: champion.name,
        champion_alias: champion.alias,
        position: position_label(current_participant),
        kills,
        deaths,
        assists,
        kda: kda_ratio(kills, deaths, assists),
        kill_participation,
        damage_share,
        total_damage,
        cs: i64_field(stats, "totalMinionsKilled") + i64_field(stats, "neutralMinionsKilled"),
        vision_score: i64_field(stats, "visionScore"),
        gold: i64_field(stats, "goldEarned"),
        items: item_ids(stats),
        tags: match_tags(
            result,
            kills,
            deaths,
            assists,
            kill_participation,
            damage_share,
            has_team_context,
        ),
        teams: participant_teams(
            participants,
            &identities,
            current_puuid,
            current_display,
            champion_catalog,
            i64_field(game, "gameDuration"),
        ),
        timeline: match_timeline(game, participants),
    })
}

fn player_identities(game: &Value) -> HashMap<i64, PlayerIdentity> {
    game.get("participantIdentities")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|identity| {
            let participant_id = i64_field(identity, "participantId");
            let player = identity.get("player").unwrap_or(&Value::Null);
            (
                participant_id,
                PlayerIdentity {
                    puuid: player
                        .get("puuid")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    display_name: player_display_name(player),
                },
            )
        })
        .collect()
}

fn participant_teams(
    participants: &[Value],
    identities: &HashMap<i64, PlayerIdentity>,
    current_puuid: Option<&str>,
    current_display: &str,
    champion_catalog: &ChampionCatalog,
    duration_seconds: i64,
) -> Vec<Vec<MatchParticipant>> {
    let team_kills = participants.iter().fold(
        HashMap::<i64, i64>::new(),
        |mut kills_by_team, participant| {
            let team_id = i64_field(participant, "teamId");
            let kills = i64_field(participant.get("stats").unwrap_or(&Value::Null), "kills");
            *kills_by_team.entry(team_id).or_default() += kills;
            kills_by_team
        },
    );

    [100, 200]
        .into_iter()
        .map(|team_id| {
            participants
                .iter()
                .filter(|participant| i64_field(participant, "teamId") == team_id)
                .take(5)
                .map(|participant| {
                    let participant_id = i64_field(participant, "participantId");
                    let team_id = i64_field(participant, "teamId");
                    let identity = identities.get(&participant_id);
                    let champion_id = i64_field(participant, "championId");
                    let champion = champion_metadata(champion_catalog, champion_id);
                    let stats = participant.get("stats").unwrap_or(&Value::Null);
                    let duration = duration_seconds.max(i64_field(stats, "timePlayed")).max(1);
                    let kills = i64_field(stats, "kills");
                    let deaths = i64_field(stats, "deaths");
                    let assists = i64_field(stats, "assists");
                    let cs = i64_field(stats, "totalMinionsKilled")
                        + i64_field(stats, "neutralMinionsKilled");
                    MatchParticipant {
                        participant_id,
                        team_id,
                        display_name: identity
                            .map(|identity| identity.display_name.clone())
                            .filter(|name| !name.is_empty())
                            .unwrap_or_else(|| "未知玩家".to_string()),
                        champion_id,
                        champion_name: champion.name,
                        champion_alias: champion.alias,
                        is_current_player: identity.is_some_and(|identity| {
                            is_current_identity(identity, current_puuid, current_display)
                        }),
                        position: position_label(participant),
                        level: i64_field(stats, "champLevel"),
                        kills,
                        deaths,
                        assists,
                        kda: kda_ratio(kills, deaths, assists),
                        kill_participation: percent(
                            kills + assists,
                            *team_kills.get(&team_id).unwrap_or(&0),
                        ),
                        total_damage: i64_field(stats, "totalDamageDealtToChampions"),
                        damage_taken: i64_field(stats, "totalDamageTaken"),
                        cs,
                        cs_per_minute: per_minute(cs, duration),
                        gold: i64_field(stats, "goldEarned"),
                        gold_per_minute: per_minute(i64_field(stats, "goldEarned"), duration),
                        vision_score: i64_field(stats, "visionScore"),
                        spell_ids: vec![
                            i64_field(participant, "spell1Id"),
                            i64_field(participant, "spell2Id"),
                        ]
                        .into_iter()
                        .filter(|spell_id| *spell_id > 0)
                        .collect(),
                        items: item_ids(stats),
                        rune_style_ids: rune_style_ids(stats),
                        perk_ids: perk_ids(stats),
                    }
                })
                .collect()
        })
        .collect()
}

fn match_timeline(game: &Value, participants: &[Value]) -> Option<MatchTimeline> {
    let frames = game
        .get("timeline")
        .and_then(|timeline| timeline.get("frames"))
        .and_then(Value::as_array)?;
    let team_by_participant = participants
        .iter()
        .map(|participant| {
            (
                i64_field(participant, "participantId"),
                i64_field(participant, "teamId"),
            )
        })
        .collect::<HashMap<_, _>>();
    let mut events = Vec::new();
    let mut gold_series = Vec::new();

    for frame in frames {
        let timestamp = i64_field(frame, "timestamp");
        if let Some(participant_frames) = frame.get("participantFrames").and_then(Value::as_object)
        {
            let mut blue_gold = 0;
            let mut red_gold = 0;
            for frame_value in participant_frames.values() {
                let participant_id = i64_field(frame_value, "participantId");
                let total_gold = i64_field(frame_value, "totalGold");
                match team_by_participant.get(&participant_id).copied() {
                    Some(100) => blue_gold += total_gold,
                    Some(200) => red_gold += total_gold,
                    _ => {}
                }
            }
            if blue_gold > 0 || red_gold > 0 {
                gold_series.push(MatchGoldFrame {
                    timestamp,
                    blue_gold,
                    red_gold,
                });
            }
        }

        if let Some(frame_events) = frame.get("events").and_then(Value::as_array) {
            for event in frame_events {
                let event_type = string_field(event, "type", "");
                if is_notable_timeline_event(&event_type) {
                    events.push(MatchTimelineEvent {
                        timestamp: i64_field(event, "timestamp"),
                        event_type,
                        participant_id: positive_i64_field(event, "participantId"),
                        killer_id: positive_i64_field(event, "killerId"),
                        victim_id: positive_i64_field(event, "victimId"),
                        assisting_participant_ids: i64_array_field(event, "assistingParticipantIds"),
                        item_id: positive_i64_field(event, "itemId"),
                        skill_slot: positive_i64_field(event, "skillSlot"),
                        team_id: positive_i64_field(event, "teamId"),
                        monster_type: string_field(event, "monsterType", ""),
                        monster_sub_type: string_field(event, "monsterSubType", ""),
                        building_type: string_field(event, "buildingType", ""),
                        lane_type: string_field(event, "laneType", ""),
                        tower_type: string_field(event, "towerType", ""),
                    });
                }
            }
        }
    }

    events.sort_by_key(|event| event.timestamp);
    events.truncate(320);
    Some(MatchTimeline {
        events,
        gold_series,
    })
}

fn is_notable_timeline_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "CHAMPION_KILL"
            | "ELITE_MONSTER_KILL"
            | "BUILDING_KILL"
            | "TURRET_PLATE_DESTROYED"
            | "WARD_PLACED"
            | "WARD_KILL"
            | "ITEM_PURCHASED"
            | "ITEM_SOLD"
            | "ITEM_DESTROYED"
            | "ITEM_UNDO"
            | "SKILL_LEVEL_UP"
    )
}

fn summarize_matches(matches: &[RecentMatch]) -> MatchHistorySummary {
    let total_games = matches.len();
    let wins = matches.iter().filter(|game| game.result == "win").count();
    let losses = total_games.saturating_sub(wins);
    let avg_kda = average(matches.iter().map(|game| game.kda));
    let avg_kill_participation = average(matches.iter().map(|game| game.kill_participation));
    let avg_damage_share = average(matches.iter().map(|game| game.damage_share));
    let mut champion_counts = HashMap::<i64, (String, Option<String>, usize, usize)>::new();

    for game in matches {
        let entry = champion_counts.entry(game.champion_id).or_insert_with(|| {
            (
                game.champion_name.clone(),
                game.champion_alias.clone(),
                0usize,
                0usize,
            )
        });
        entry.2 += 1;
        if game.result == "win" {
            entry.3 += 1;
        }
    }

    let mut favorite_champions = champion_counts
        .into_iter()
        .map(
            |(champion_id, (champion_name, champion_alias, games, wins))| FavoriteChampion {
                champion_id,
                champion_name,
                champion_alias,
                games,
                wins,
            },
        )
        .collect::<Vec<_>>();
    favorite_champions.sort_by(|a, b| b.games.cmp(&a.games).then_with(|| b.wins.cmp(&a.wins)));
    favorite_champions.truncate(5);

    MatchHistorySummary {
        total_games,
        wins,
        losses,
        win_rate: percent(wins as i64, total_games as i64),
        avg_kda,
        avg_kill_participation,
        avg_damage_share,
        favorite_champions,
    }
}

fn is_current_identity(
    identity: &PlayerIdentity,
    current_puuid: Option<&str>,
    current_display: &str,
) -> bool {
    current_puuid.is_some_and(|puuid| identity.puuid.as_deref() == Some(puuid))
        || normalize_name(&identity.display_name) == normalize_name(current_display)
}

fn summoner_display_name(summoner: &Value) -> String {
    let game_name = summoner.get("gameName").and_then(Value::as_str);
    let tag_line = summoner.get("tagLine").and_then(Value::as_str);
    match (game_name, tag_line) {
        (Some(game_name), Some(tag_line)) if !game_name.is_empty() && !tag_line.is_empty() => {
            format!("{game_name}#{tag_line}")
        }
        (Some(game_name), _) if !game_name.is_empty() => game_name.to_string(),
        _ => summoner
            .get("displayName")
            .or_else(|| summoner.get("summonerName"))
            .and_then(Value::as_str)
            .unwrap_or("当前玩家")
            .to_string(),
    }
}

fn player_display_name(player: &Value) -> String {
    let game_name = player.get("gameName").and_then(Value::as_str);
    let tag_line = player.get("tagLine").and_then(Value::as_str);
    match (game_name, tag_line) {
        (Some(game_name), Some(tag_line)) if !game_name.is_empty() && !tag_line.is_empty() => {
            format!("{game_name}#{tag_line}")
        }
        (Some(game_name), _) if !game_name.is_empty() => game_name.to_string(),
        _ => player
            .get("summonerName")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    }
}

struct ChampionMetadata {
    name: String,
    alias: Option<String>,
}

fn champion_metadata(catalog: &ChampionCatalog, champion_id: i64) -> ChampionMetadata {
    catalog.get(champion_id).map_or_else(
        || ChampionMetadata {
            name: format!("英雄 {champion_id}"),
            alias: None,
        },
        |champion| ChampionMetadata {
            name: champion.name.clone(),
            alias: Some(champion.alias.clone()),
        },
    )
}

fn position_label(participant: &Value) -> String {
    let timeline = participant.get("timeline").unwrap_or(&Value::Null);
    let lane = string_field(timeline, "lane", "");
    let role = string_field(timeline, "role", "");
    match (lane.as_str(), role.as_str()) {
        ("TOP", _) => "上路",
        ("JUNGLE", _) => "打野",
        ("MIDDLE", _) | ("MID", _) => "中路",
        ("BOTTOM", "DUO_CARRY") => "下路",
        ("BOTTOM", "DUO_SUPPORT") => "辅助",
        ("BOTTOM", _) => "下路",
        _ => "位置未知",
    }
    .to_string()
}

fn queue_label(queue_id: i64) -> &'static str {
    match queue_id {
        400 => "匹配 自选",
        420 => "单双排",
        430 => "匹配 盲选",
        440 => "灵活排位",
        450 => "极地大乱斗",
        700 => "冠军杯",
        1700 => "斗魂竞技场",
        1900 => "无限火力",
        _ => "召唤师峡谷",
    }
}

fn ended_at_label(game: &Value) -> String {
    let creation_ms = game
        .get("gameCreation")
        .or_else(|| game.get("gameCreationTime"))
        .and_then(Value::as_i64);
    if let Some(creation_ms) = creation_ms {
        if creation_ms > 0 {
            return format!("{} 前", rough_age_label(creation_ms));
        }
    }

    game.get("gameCreationDate")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| "最近".to_string())
}

fn rough_age_label(creation_ms: i64) -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(creation_ms);
    let elapsed_seconds = ((now_ms - creation_ms) / 1000).max(0);
    if elapsed_seconds < 3600 {
        return format!("{} 分钟", (elapsed_seconds / 60).max(1));
    }
    if elapsed_seconds < 86_400 {
        return format!("{} 小时", elapsed_seconds / 3600);
    }
    format!("{} 天", elapsed_seconds / 86_400)
}

fn match_tags(
    result: &str,
    kills: i64,
    deaths: i64,
    assists: i64,
    kill_participation: f64,
    damage_share: f64,
    has_team_context: bool,
) -> Vec<String> {
    let mut tags = vec![if result == "win" { "胜利" } else { "失败" }.to_string()];
    if has_team_context && kill_participation >= 60.0 {
        tags.push("高参团".to_string());
    }
    if has_team_context && damage_share >= 30.0 {
        tags.push("高伤害".to_string());
    }
    if deaths == 0 && kills + assists > 0 {
        tags.push("零阵亡".to_string());
    }
    if kda_ratio(kills, deaths, assists) >= 5.0 {
        tags.push("高KDA".to_string());
    }
    tags.truncate(4);
    tags
}

fn item_ids(stats: &Value) -> Vec<i64> {
    (0..=6)
        .filter_map(|index| {
            let item_id = i64_field(stats, &format!("item{index}"));
            (item_id > 0).then_some(item_id)
        })
        .collect()
}

fn rune_style_ids(stats: &Value) -> Vec<i64> {
    ["perkPrimaryStyle", "perkSubStyle"]
        .into_iter()
        .filter_map(|key| {
            let style_id = i64_field(stats, key);
            (style_id > 0).then_some(style_id)
        })
        .collect()
}

fn perk_ids(stats: &Value) -> Vec<i64> {
    (0..=5)
        .filter_map(|index| {
            let perk_id = i64_field(stats, &format!("perk{index}"));
            (perk_id > 0).then_some(perk_id)
        })
        .collect()
}

fn win_field(stats: &Value) -> bool {
    if let Some(win) = stats.get("win").and_then(Value::as_bool) {
        return win;
    }
    stats
        .get("win")
        .and_then(Value::as_str)
        .is_some_and(|value| matches!(value, "Win" | "win" | "true"))
}

fn i64_field(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or_default()
}

fn positive_i64_field(value: &Value, key: &str) -> Option<i64> {
    let value = i64_field(value, key);
    (value > 0).then_some(value)
}

fn i64_array_field(value: &Value, key: &str) -> Vec<i64> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .filter(|value| *value > 0)
        .collect()
}

fn string_field(value: &Value, key: &str, fallback: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn percent(numerator: i64, denominator: i64) -> f64 {
    if denominator <= 0 {
        return 0.0;
    }
    (((numerator as f64 / denominator as f64) * 1000.0).round() / 10.0).clamp(0.0, 100.0)
}

fn kda_ratio(kills: i64, deaths: i64, assists: i64) -> f64 {
    let divisor = deaths.max(1) as f64;
    (((kills + assists) as f64 / divisor) * 100.0).round() / 100.0
}

fn per_minute(value: i64, duration_seconds: i64) -> f64 {
    if duration_seconds <= 0 {
        return 0.0;
    }
    ((value as f64 / duration_seconds as f64) * 600.0).round() / 10.0
}

fn average(values: impl Iterator<Item = f64>) -> f64 {
    let mut total = 0.0;
    let mut count = 0usize;
    for value in values {
        total += value;
        count += 1;
    }
    if count == 0 {
        return 0.0;
    }
    ((total / count as f64) * 10.0).round() / 10.0
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn summarizes_recent_match_for_current_player() {
        let catalog = ChampionCatalog::from_lcu_summary(&json!([
            { "id": 103, "alias": "Ahri", "name": "阿狸" },
            { "id": 64, "alias": "LeeSin", "name": "盲僧" }
        ]));
        let summoner = json!({
            "puuid": "self-puuid",
            "gameName": "别生气了嘛",
            "tagLine": "62047"
        });
        let value = json!({
            "games": {
                "games": [{
                    "gameId": 1,
                    "queueId": 420,
                    "gameDuration": 1800,
                    "participants": [
                        {
                            "participantId": 1,
                            "teamId": 100,
                            "championId": 103,
                            "stats": {
                                "win": true,
                                "kills": 8,
                                "deaths": 2,
                                "assists": 10,
                                "totalDamageDealtToChampions": 24000,
                                "totalMinionsKilled": 210,
                                "neutralMinionsKilled": 4,
                                "visionScore": 26,
                                "goldEarned": 12800,
                                "item0": 1056
                            },
                            "timeline": { "lane": "MIDDLE", "role": "SOLO" }
                        },
                        {
                            "participantId": 2,
                            "teamId": 100,
                            "championId": 64,
                            "stats": {
                                "kills": 12,
                                "deaths": 4,
                                "assists": 8,
                                "totalDamageDealtToChampions": 12000
                            }
                        }
                    ],
                    "participantIdentities": [
                        {
                            "participantId": 1,
                            "player": {
                                "puuid": "self-puuid",
                                "gameName": "别生气了嘛",
                                "tagLine": "62047"
                            }
                        },
                        {
                            "participantId": 2,
                            "player": { "summonerName": "队友" }
                        }
                    ]
                }]
            }
        });

        let history = summarize_match_history(&value, &summoner, &catalog).unwrap();

        assert_eq!(history.summary.total_games, 1);
        assert_eq!(history.summary.wins, 1);
        assert_eq!(history.matches[0].champion_name, "阿狸");
        assert_eq!(history.matches[0].position, "中路");
        assert_eq!(history.matches[0].kill_participation, 90.0);
        assert_eq!(history.matches[0].damage_share, 66.7);
    }

    #[test]
    fn requests_game_detail_when_participant_stats_are_missing() {
        let game = json!({
            "gameId": 123,
            "participants": (0..10)
                .map(|index| json!({
                    "participantId": index + 1,
                    "teamId": if index < 5 { 100 } else { 200 },
                    "championId": 103
                }))
                .collect::<Vec<_>>(),
            "participantIdentities": (0..10)
                .map(|index| json!({
                    "participantId": index + 1,
                    "player": { "summonerName": format!("player{index}") }
                }))
                .collect::<Vec<_>>()
        });

        assert!(should_request_game_detail(&game));
    }
}
