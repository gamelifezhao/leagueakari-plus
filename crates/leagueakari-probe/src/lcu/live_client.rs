use serde::Deserialize;
use thiserror::Error;

use super::{
    champions::ChampionCatalog,
    models::{DraftPlayer, DraftState},
};

const LIVE_CLIENT_ALL_GAME_DATA_URL: &str = "https://127.0.0.1:2999/liveclientdata/allgamedata";

#[derive(Debug, Error)]
pub enum LiveClientError {
    #[error("failed to build live client HTTP client: {0}")]
    Build(#[source] reqwest::Error),
    #[error("live client request failed for {url}: {source}")]
    Request { url: String, source: reqwest::Error },
    #[error("live client returned {status} for {url}: {body}")]
    Status {
        url: String,
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("failed to parse live client JSON from {url}: {source}")]
    Json {
        url: String,
        source: serde_json::Error,
    },
}

#[derive(Debug, Deserialize)]
struct LiveClientAllGameData {
    #[serde(rename = "activePlayer")]
    active_player: Option<LiveClientActivePlayer>,
    #[serde(rename = "allPlayers")]
    all_players: Vec<LiveClientPlayer>,
}

#[derive(Debug, Deserialize)]
struct LiveClientActivePlayer {
    #[serde(rename = "riotId")]
    riot_id: Option<String>,
    #[serde(rename = "riotIdGameName")]
    riot_id_game_name: Option<String>,
    #[serde(rename = "riotIdTagLine")]
    riot_id_tag_line: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LiveClientPlayer {
    #[serde(rename = "rawChampionName")]
    raw_champion_name: Option<String>,
    #[serde(rename = "championName")]
    champion_name: Option<String>,
    position: Option<String>,
    team: String,
    #[serde(rename = "riotId")]
    riot_id: Option<String>,
    #[serde(rename = "riotIdGameName")]
    riot_id_game_name: Option<String>,
    #[serde(rename = "riotIdTagLine")]
    riot_id_tag_line: Option<String>,
}

pub async fn fetch_in_progress_draft(champion_catalog: &ChampionCatalog) -> Option<DraftState> {
    fetch_in_progress_draft_result(champion_catalog)
        .await
        .ok()
        .flatten()
}

pub async fn fetch_in_progress_draft_result(
    champion_catalog: &ChampionCatalog,
) -> Result<Option<DraftState>, LiveClientError> {
    let http = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .no_proxy()
        .connect_timeout(std::time::Duration::from_millis(800))
        .timeout(std::time::Duration::from_millis(2000))
        .build()
        .map_err(LiveClientError::Build)?;
    let url = LIVE_CLIENT_ALL_GAME_DATA_URL.to_string();
    let response = http
        .get(&url)
        .send()
        .await
        .map_err(|source| LiveClientError::Request {
            url: url.clone(),
            source,
        })?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_else(|_| String::new());
        return Err(LiveClientError::Status { url, status, body });
    }
    let body = response
        .text()
        .await
        .map_err(|source| LiveClientError::Request {
            url: url.clone(),
            source,
        })?;
    let data = serde_json::from_str::<LiveClientAllGameData>(&body)
        .map_err(|source| LiveClientError::Json { url, source })?;

    Ok(parse_live_client_draft(&data, champion_catalog))
}

fn parse_live_client_draft(
    data: &LiveClientAllGameData,
    champion_catalog: &ChampionCatalog,
) -> Option<DraftState> {
    let active_riot_id = data.active_player.as_ref().and_then(active_player_key);
    let local_team = active_riot_id
        .as_deref()
        .and_then(|key| {
            data.all_players
                .iter()
                .find(|player| player_key(player).as_deref() == Some(key))
        })
        .map(|player| player.team.as_str())
        .or_else(|| data.all_players.first().map(|player| player.team.as_str()))?;

    let mut my_team = Vec::new();
    let mut their_team = Vec::new();
    let mut local_player_cell_id = None;

    for player in &data.all_players {
        let champion_id = champion_id_from_live_player(player, champion_catalog);
        let draft_player = DraftPlayer {
            cell_id: if player.team == local_team {
                my_team.len() as i64
            } else {
                their_team.len() as i64 + 5
            },
            champion_id,
            assigned_position: player.position.as_deref().and_then(position_label),
            summoner_id: None,
        };

        if active_riot_id.as_deref() == player_key(player).as_deref() {
            local_player_cell_id = Some(draft_player.cell_id);
        }

        if player.team == local_team {
            my_team.push(draft_player);
        } else {
            their_team.push(draft_player);
        }
    }

    if my_team.is_empty() && their_team.is_empty() {
        return None;
    }

    Some(DraftState {
        connected: true,
        gameflow: "InProgress".to_string(),
        local_player_cell_id,
        my_team,
        their_team,
        bans: Vec::new(),
    })
}

fn champion_id_from_live_player(
    player: &LiveClientPlayer,
    champion_catalog: &ChampionCatalog,
) -> Option<i64> {
    player
        .raw_champion_name
        .as_deref()
        .and_then(raw_champion_alias)
        .and_then(|alias| champion_catalog.find_by_alias_or_name(alias))
        .map(|champion| champion.id)
        .or_else(|| {
            player.champion_name.as_deref().and_then(|name| {
                champion_catalog
                    .find_by_alias_or_name(name)
                    .map(|champion| champion.id)
            })
        })
}

fn raw_champion_alias(raw_champion_name: &str) -> Option<&str> {
    raw_champion_name
        .strip_prefix("game_character_displayname_")
        .or_else(|| {
            raw_champion_name
                .strip_prefix("Character_")?
                .strip_suffix("_Name")
        })
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

fn active_player_key(active_player: &LiveClientActivePlayer) -> Option<String> {
    riot_id_key(
        active_player.riot_id.as_deref(),
        active_player.riot_id_game_name.as_deref(),
        active_player.riot_id_tag_line.as_deref(),
    )
}

fn player_key(player: &LiveClientPlayer) -> Option<String> {
    riot_id_key(
        player.riot_id.as_deref(),
        player.riot_id_game_name.as_deref(),
        player.riot_id_tag_line.as_deref(),
    )
}

fn riot_id_key(
    riot_id: Option<&str>,
    riot_id_game_name: Option<&str>,
    riot_id_tag_line: Option<&str>,
) -> Option<String> {
    if let Some(riot_id) = riot_id.filter(|value| !value.is_empty() && *value != "#") {
        return Some(riot_id.to_string());
    }

    let game_name = riot_id_game_name.filter(|value| !value.is_empty())?;
    let tag_line = riot_id_tag_line.filter(|value| !value.is_empty())?;
    Some(format!("{game_name}#{tag_line}"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_live_client_players_into_draft_state() {
        let catalog = ChampionCatalog::from_lcu_summary(&json!([
            { "id": 29, "alias": "Twitch", "name": "瘟疫之源" },
            { "id": 103, "alias": "Ahri", "name": "九尾妖狐" },
            { "id": 412, "alias": "Thresh", "name": "魂锁典狱长" }
        ]));
        let data = serde_json::from_value::<LiveClientAllGameData>(json!({
            "activePlayer": {
                "riotId": "me#12345",
                "riotIdGameName": "me",
                "riotIdTagLine": "12345"
            },
            "allPlayers": [
                {
                    "rawChampionName": "game_character_displayname_Twitch",
                    "championName": "瘟疫之源",
                    "position": "BOTTOM",
                    "team": "ORDER",
                    "riotId": "me#12345",
                    "riotIdGameName": "me",
                    "riotIdTagLine": "12345"
                },
                {
                    "rawChampionName": "game_character_displayname_Ahri",
                    "championName": "九尾妖狐",
                    "position": "MIDDLE",
                    "team": "ORDER",
                    "riotId": "ally#12345",
                    "riotIdGameName": "ally",
                    "riotIdTagLine": "12345"
                },
                {
                    "rawChampionName": "game_character_displayname_Thresh",
                    "championName": "魂锁典狱长",
                    "position": "UTILITY",
                    "team": "CHAOS",
                    "riotId": "enemy#12345",
                    "riotIdGameName": "enemy",
                    "riotIdTagLine": "12345"
                }
            ]
        }))
        .unwrap();

        let draft = parse_live_client_draft(&data, &catalog).unwrap();

        assert_eq!(draft.gameflow, "InProgress");
        assert_eq!(draft.local_player_cell_id, Some(0));
        assert_eq!(draft.my_team.len(), 2);
        assert_eq!(draft.my_team[0].champion_id, Some(29));
        assert_eq!(
            draft.my_team[0].assigned_position.as_deref(),
            Some("bottom")
        );
        assert_eq!(draft.their_team.len(), 1);
        assert_eq!(draft.their_team[0].champion_id, Some(412));
        assert!(draft.bans.is_empty());
    }

    #[test]
    fn parses_character_name_raw_champion_format() {
        let catalog = ChampionCatalog::from_lcu_summary(&json!([
            { "id": 266, "alias": "Aatrox", "name": "暗裔剑魔" },
            { "id": 103, "alias": "Ahri", "name": "九尾妖狐" }
        ]));
        let data = serde_json::from_value::<LiveClientAllGameData>(json!({
            "activePlayer": {
                "riotId": "me#12345",
                "riotIdGameName": "me",
                "riotIdTagLine": "12345"
            },
            "allPlayers": [
                {
                    "rawChampionName": "game_character_displayname_Ahri",
                    "championName": "九尾妖狐",
                    "position": "MIDDLE",
                    "team": "ORDER",
                    "riotId": "me#12345",
                    "riotIdGameName": "me",
                    "riotIdTagLine": "12345"
                },
                {
                    "rawChampionName": "Character_Aatrox_Name",
                    "championName": "暗裔剑魔",
                    "position": "TOP",
                    "team": "CHAOS",
                    "riotId": "enemy#12345",
                    "riotIdGameName": "enemy",
                    "riotIdTagLine": "12345"
                }
            ]
        }))
        .unwrap();

        let draft = parse_live_client_draft(&data, &catalog).unwrap();

        assert_eq!(draft.their_team[0].champion_id, Some(266));
        assert_eq!(
            draft.their_team[0].assigned_position.as_deref(),
            Some("top")
        );
    }
}
