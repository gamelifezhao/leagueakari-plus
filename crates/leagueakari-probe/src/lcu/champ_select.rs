use serde_json::Value;

use super::models::{DraftBan, DraftPlayer, DraftState};

pub fn parse_draft_state(gameflow: &str, session: &Value) -> DraftState {
    let local_player_cell_id = session.get("localPlayerCellId").and_then(Value::as_i64);

    let mut my_team = parse_players(session.get("myTeam"));
    let mut their_team = parse_players(session.get("theirTeam"));
    merge_action_picks(session.get("actions"), &mut my_team, &mut their_team);
    let mut bans = parse_bans(session.get("bans"));
    if bans.is_empty() {
        bans = parse_action_bans(session.get("actions"));
    }

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
            puuid: player
                .get("puuid")
                .and_then(Value::as_str)
                .filter(|puuid| !puuid.is_empty())
                .map(ToOwned::to_owned),
            display_name: player
                .get("displayName")
                .and_then(Value::as_str)
                .or_else(|| player.get("summonerName").and_then(Value::as_str))
                .filter(|name| !name.is_empty())
                .map(ToOwned::to_owned),
            team_side: None,
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

fn parse_action_bans(value: Option<&Value>) -> Vec<DraftBan> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .flatten()
        .filter(|action| action.get("type").and_then(Value::as_str) == Some("ban"))
        .filter(|action| {
            action
                .get("completed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|action| {
            positive_i64(action.get("championId")).map(|champion_id| DraftBan {
                champion_id,
                team_id: action
                    .get("isAllyAction")
                    .and_then(Value::as_bool)
                    .map(|is_ally| if is_ally { 100 } else { 200 }),
            })
        })
        .collect()
}

fn merge_action_picks(
    value: Option<&Value>,
    my_team: &mut Vec<DraftPlayer>,
    their_team: &mut Vec<DraftPlayer>,
) {
    let Some(actions) = value.and_then(Value::as_array) else {
        return;
    };

    for action in actions
        .iter()
        .filter_map(Value::as_array)
        .flatten()
        .filter(|action| action.get("type").and_then(Value::as_str) == Some("pick"))
        .filter(|action| {
            action
                .get("completed")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
    {
        let Some(champion_id) = positive_i64(action.get("championId")) else {
            continue;
        };
        let cell_id = action
            .get("actorCellId")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let target = if action
            .get("isAllyAction")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            &mut *my_team
        } else {
            &mut *their_team
        };

        upsert_action_pick(target, cell_id, champion_id);
    }
}

fn upsert_action_pick(team: &mut Vec<DraftPlayer>, cell_id: i64, champion_id: i64) {
    if let Some(player) = team.iter_mut().find(|player| player.cell_id == cell_id) {
        player.champion_id = Some(champion_id);
        return;
    }

    team.push(DraftPlayer {
        cell_id,
        champion_id: Some(champion_id),
        assigned_position: None,
        summoner_id: None,
        puuid: None,
        display_name: None,
        team_side: None,
    });
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

    #[test]
    fn parses_bans_from_actions_when_bans_field_is_empty() {
        let session = json!({
            "actions": [
                [
                    {
                        "championId": 63,
                        "completed": true,
                        "isAllyAction": true,
                        "type": "ban"
                    },
                    {
                        "championId": 157,
                        "completed": true,
                        "isAllyAction": false,
                        "type": "ban"
                    },
                    {
                        "championId": 103,
                        "completed": true,
                        "isAllyAction": true,
                        "type": "pick"
                    }
                ]
            ],
            "bans": {
                "myTeamBans": [],
                "theirTeamBans": []
            }
        });

        let draft = parse_draft_state("ChampSelect", &session);

        assert_eq!(draft.bans.len(), 2);
        assert_eq!(draft.bans[0].champion_id, 63);
        assert_eq!(draft.bans[0].team_id, Some(100));
        assert_eq!(draft.bans[1].champion_id, 157);
        assert_eq!(draft.bans[1].team_id, Some(200));
    }

    #[test]
    fn merges_public_picks_from_actions() {
        let session = json!({
            "myTeam": [],
            "theirTeam": [],
            "actions": [
                [
                    {
                        "actorCellId": 1,
                        "championId": 103,
                        "completed": true,
                        "isAllyAction": true,
                        "type": "pick"
                    },
                    {
                        "actorCellId": 6,
                        "championId": 22,
                        "completed": true,
                        "isAllyAction": false,
                        "type": "pick"
                    }
                ]
            ]
        });

        let draft = parse_draft_state("ChampSelect", &session);

        assert_eq!(draft.my_team.len(), 1);
        assert_eq!(draft.my_team[0].champion_id, Some(103));
        assert_eq!(draft.their_team.len(), 1);
        assert_eq!(draft.their_team[0].champion_id, Some(22));
    }

    #[test]
    fn parses_tencent_style_ban_and_pick_actions() {
        let session = json!({
            "localPlayerCellId": 2,
            "myTeam": [],
            "theirTeam": [],
            "actions": [
                [
                    { "actorCellId": 0, "championId": 63, "completed": true, "isAllyAction": true, "type": "ban" },
                    { "actorCellId": 1, "championId": 104, "completed": true, "isAllyAction": true, "type": "ban" },
                    { "actorCellId": 2, "championId": 121, "completed": true, "isAllyAction": true, "type": "ban" },
                    { "actorCellId": 3, "championId": 893, "completed": true, "isAllyAction": true, "type": "ban" },
                    { "actorCellId": 4, "championId": 11, "completed": true, "isAllyAction": true, "type": "ban" },
                    { "actorCellId": 5, "championId": 157, "completed": true, "isAllyAction": false, "type": "ban" },
                    { "actorCellId": 6, "championId": 89, "completed": true, "isAllyAction": false, "type": "ban" },
                    { "actorCellId": 7, "championId": 56, "completed": true, "isAllyAction": false, "type": "ban" },
                    { "actorCellId": 8, "championId": 800, "completed": true, "isAllyAction": false, "type": "ban" },
                    { "actorCellId": 9, "championId": 54, "completed": true, "isAllyAction": false, "type": "ban" }
                ],
                [
                    { "actorCellId": 0, "championId": 103, "completed": true, "isAllyAction": true, "type": "pick" },
                    { "actorCellId": 5, "championId": 22, "completed": true, "isAllyAction": false, "type": "pick" }
                ]
            ],
            "bans": {
                "myTeamBans": [],
                "theirTeamBans": []
            }
        });

        let draft = parse_draft_state("ChampSelect", &session);

        assert_eq!(draft.local_player_cell_id, Some(2));
        assert_eq!(draft.bans.len(), 10);
        assert_eq!(
            draft
                .bans
                .iter()
                .filter(|ban| ban.team_id == Some(100))
                .count(),
            5
        );
        assert_eq!(
            draft
                .bans
                .iter()
                .filter(|ban| ban.team_id == Some(200))
                .count(),
            5
        );
        assert_eq!(draft.my_team[0].champion_id, Some(103));
        assert_eq!(draft.their_team[0].champion_id, Some(22));
    }
}
