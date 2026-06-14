use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftState {
    pub connected: bool,
    pub gameflow: String,
    pub local_player_cell_id: Option<i64>,
    pub my_team: Vec<DraftPlayer>,
    pub their_team: Vec<DraftPlayer>,
    pub bans: Vec<DraftBan>,
}

impl DraftState {
    pub fn empty(gameflow: String) -> Self {
        Self {
            connected: true,
            gameflow,
            local_player_cell_id: None,
            my_team: Vec::new(),
            their_team: Vec::new(),
            bans: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftPlayer {
    pub cell_id: i64,
    pub champion_id: Option<i64>,
    pub assigned_position: Option<String>,
    pub summoner_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct DraftPlayerIdentity {
    pub cell_id: i64,
    pub champion_id: Option<i64>,
    pub assigned_position: Option<String>,
    pub puuid: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftBan {
    pub champion_id: i64,
    pub team_id: Option<i64>,
}
