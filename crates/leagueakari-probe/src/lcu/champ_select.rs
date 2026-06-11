use serde_json::Value;

use super::models::{DraftBan, DraftPlayer, DraftState};

pub fn parse_draft_state(gameflow: &str, session: &Value) -> DraftState {
    let local_player_cell_id = session.get("localPlayerCellId").and_then(Value::as_i64);

    let my_team = parse_players(session.get("myTeam"));
    let their_team = parse_players(session.get("theirTeam"));
    let bans = parse_bans(session.get("bans"));

    DraftState {
        connected: true,
        gameflow: gameflow.to_string(),
        local_player_cell_id,
        my_team,
        their_team,
        bans,
    }
}

fn parse_players(value: Option<&Value>) -> Vec<DraftPlayer> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|player| DraftPlayer {
            cell_id: player
                .get("cellId")
                .and_then(Value::as_i64)
                .unwrap_or_default(),
            champion_id: positive_i64(player.get("championId")),
            assigned_position: player
                .get("assignedPosition")
                .and_then(Value::as_str)
                .filter(|position| !position.is_empty())
                .map(ToOwned::to_owned),
            summoner_id: positive_i64(player.get("summonerId")),
        })
        .collect()
}

fn parse_bans(value: Option<&Value>) -> Vec<DraftBan> {
    let mut bans = Vec::new();

    if let Some(my_team_bans) = value
        .and_then(|bans| bans.get("myTeamBans"))
        .and_then(Value::as_array)
    {
        bans.extend(parse_ban_ids(my_team_bans, Some(100)));
    }

    if let Some(their_team_bans) = value
        .and_then(|bans| bans.get("theirTeamBans"))
        .and_then(Value::as_array)
    {
        bans.extend(parse_ban_ids(their_team_bans, Some(200)));
    }

    bans
}

fn parse_ban_ids(values: &[Value], team_id: Option<i64>) -> impl Iterator<Item = DraftBan> + '_ {
    values.iter().filter_map(move |value| {
        positive_i64(Some(value)).map(|champion_id| DraftBan {
            champion_id,
            team_id,
        })
    })
}

fn positive_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(Value::as_i64).filter(|id| *id > 0)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_session_into_draft_state() {
        let session = json!({
            "localPlayerCellId": 1,
            "myTeam": [
                {
                    "cellId": 1,
                    "championId": 103,
                    "assignedPosition": "middle",
                    "summonerId": 1001
                },
                {
                    "cellId": 2,
                    "championId": 0,
                    "assignedPosition": "",
                    "summonerId": 0
                }
            ],
            "theirTeam": [
                {
                    "cellId": 6,
                    "championId": 22,
                    "assignedPosition": "bottom",
                    "summonerId": 2001
                }
            ],
            "bans": {
                "myTeamBans": [64, 0],
                "theirTeamBans": [238]
            }
        });

        let draft = parse_draft_state("ChampSelect", &session);

        assert_eq!(draft.local_player_cell_id, Some(1));
        assert_eq!(draft.my_team.len(), 2);
        assert_eq!(draft.my_team[0].champion_id, Some(103));
        assert_eq!(draft.my_team[1].champion_id, None);
        assert_eq!(
            draft.their_team[0].assigned_position.as_deref(),
            Some("bottom")
        );
        assert_eq!(draft.bans.len(), 2);
    }
}
