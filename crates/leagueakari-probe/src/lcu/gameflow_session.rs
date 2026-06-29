use serde_json::Value;

use super::models::{DraftPlayer, DraftState};

pub fn parse_in_progress_draft(session: &Value, current_puuid: Option<&str>) -> Option<DraftState> {
    let game_data = session.get("gameData")?;
    let team_one = game_data.get("teamOne").and_then(Value::as_array)?;
    let team_two = game_data.get("teamTwo").and_then(Value::as_array)?;
    if team_one.is_empty() && team_two.is_empty() {
        return None;
    }

    let local_team_is_one = current_puuid
        .and_then(|puuid| {
            if team_one
                .iter()
                .any(|player| player_puuid(player) == Some(puuid))
            {
                Some(true)
            } else if team_two
                .iter()
                .any(|player| player_puuid(player) == Some(puuid))
            {
                Some(false)
            } else {
                None
            }
        })
        .unwrap_or(true);

    let (local_team, enemy_team, local_side, enemy_side) = if local_team_is_one {
        (team_one, team_two, "teamOne", "teamTwo")
    } else {
        (team_two, team_one, "teamTwo", "teamOne")
    };

    let my_team = parse_team(local_team, 0, local_side);
    let their_team = parse_team(enemy_team, 5, enemy_side);
    let local_player_cell_id = current_puuid.and_then(|puuid| {
        my_team
            .iter()
            .find(|player| player.puuid.as_deref() == Some(puuid))
            .map(|player| player.cell_id)
    });

    Some(DraftState {
        connected: true,
        gameflow: "InProgress".to_string(),
        local_player_cell_id,
        my_team,
        their_team,
        bans: Vec::new(),
    })
}

fn parse_team(players: &[Value], cell_offset: i64, team_side: &str) -> Vec<DraftPlayer> {
    players
        .iter()
        .enumerate()
        .map(|(index, player)| DraftPlayer {
            cell_id: cell_offset + index as i64,
            champion_id: positive_i64(player.get("championId")),
            assigned_position: player
                .get("selectedPosition")
                .and_then(Value::as_str)
                .and_then(position_label),
            summoner_id: positive_i64(player.get("summonerId")),
            puuid: player_puuid(player).map(ToOwned::to_owned),
            display_name: display_name_from_player(player),
            team_side: Some(team_side.to_string()),
        })
        .collect()
}

fn player_puuid(player: &Value) -> Option<&str> {
    player
        .get("puuid")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
}

fn display_name_from_player(player: &Value) -> Option<String> {
    let game_name = player
        .get("gameName")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    let tag_line = player
        .get("tagLine")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    match (game_name, tag_line) {
        (Some(game_name), Some(tag_line)) => return Some(format!("{game_name}#{tag_line}")),
        (Some(game_name), None) => return Some(game_name.to_string()),
        _ => {}
    }

    player
        .get("riotId")
        .and_then(Value::as_str)
        .or_else(|| player.get("summonerName").and_then(Value::as_str))
        .or_else(|| player.get("summonerInternalName").and_then(Value::as_str))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn positive_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(Value::as_i64).filter(|value| *value > 0)
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
    fn parses_gameflow_session_into_local_and_enemy_teams() {
        let session = json!({
            "gameData": {
                "teamOne": [
                    { "puuid": "me", "championId": 103, "selectedPosition": "MIDDLE", "summonerId": 1, "gameName": "Me", "tagLine": "CN1" },
                    { "puuid": "ally", "championId": 64, "selectedPosition": "JUNGLE", "summonerId": 2, "summonerName": "Ally" }
                ],
                "teamTwo": [
                    { "puuid": "enemy", "championId": 266, "selectedPosition": "TOP", "summonerId": 6, "gameName": "Enemy", "tagLine": "CN2" }
                ]
            }
        });

        let draft = parse_in_progress_draft(&session, Some("me")).unwrap();

        assert_eq!(draft.local_player_cell_id, Some(0));
        assert_eq!(draft.my_team.len(), 2);
        assert_eq!(draft.their_team.len(), 1);
        assert_eq!(draft.my_team[0].display_name.as_deref(), Some("Me#CN1"));
        assert_eq!(
            draft.their_team[0].display_name.as_deref(),
            Some("Enemy#CN2")
        );
        assert_eq!(draft.their_team[0].team_side.as_deref(), Some("teamTwo"));
    }
}
