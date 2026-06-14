use std::collections::BTreeMap;

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use super::{
    analysis::CompositionAnalysis,
    champions::{ChampionCatalog, ChampionInfo},
    connection::LcuConnection,
    models::DraftState,
    teammate_performance::TeammatePerformance,
};

#[derive(Debug, Serialize)]
struct JsonEvent<'a, T> {
    event: &'a str,
    payload: T,
}

#[derive(Debug, Serialize)]
pub struct ConnectionSummary {
    source: String,
    path: String,
    pid: Option<u32>,
    port: u16,
    protocol: String,
    token_hidden: bool,
}

#[derive(Debug, Serialize)]
pub struct SummonerSummary {
    available: bool,
    profile_icon_id: Option<i64>,
    summoner_level: Option<i64>,
    account_fields_hidden: bool,
}

#[derive(Debug, Serialize)]
pub struct GameflowPhase<'a> {
    phase: &'a Value,
}

#[derive(Debug, Serialize)]
pub struct DraftSnapshot<'a> {
    source: &'a str,
    lcu_event_type: Option<&'a str>,
    draft_state: &'a DraftState,
    analysis: &'a CompositionAnalysis,
    champion_names: BTreeMap<i64, &'a ChampionInfo>,
    teammate_performance: &'a [TeammatePerformance],
}

#[derive(Debug, Serialize)]
pub struct StatusMessage<'a> {
    status: &'a str,
    message: &'a str,
}

pub fn connection_summary(connection: &LcuConnection) -> ConnectionSummary {
    ConnectionSummary {
        source: connection.source.clone(),
        path: connection.path.display().to_string(),
        pid: connection.pid,
        port: connection.port,
        protocol: connection.protocol.clone(),
        token_hidden: true,
    }
}

pub fn summoner_summary(summoner: Option<&Value>) -> SummonerSummary {
    SummonerSummary {
        available: summoner.is_some(),
        profile_icon_id: summoner
            .and_then(|value| value.get("profileIconId"))
            .and_then(Value::as_i64),
        summoner_level: summoner
            .and_then(|value| value.get("summonerLevel"))
            .and_then(Value::as_i64),
        account_fields_hidden: true,
    }
}

pub fn gameflow_phase(phase: &Value) -> GameflowPhase<'_> {
    GameflowPhase { phase }
}

pub fn draft_snapshot<'a>(
    source: &'a str,
    lcu_event_type: Option<&'a str>,
    draft_state: &'a DraftState,
    analysis: &'a CompositionAnalysis,
    champion_catalog: &'a ChampionCatalog,
    teammate_performance: &'a [TeammatePerformance],
) -> DraftSnapshot<'a> {
    DraftSnapshot {
        source,
        lcu_event_type,
        draft_state,
        analysis,
        champion_names: champion_names_for(draft_state, champion_catalog),
        teammate_performance,
    }
}

fn champion_names_for<'a>(
    draft_state: &DraftState,
    champion_catalog: &'a ChampionCatalog,
) -> BTreeMap<i64, &'a ChampionInfo> {
    let mut names = BTreeMap::new();

    for champion_id in draft_state
        .my_team
        .iter()
        .chain(draft_state.their_team.iter())
        .filter_map(|player| player.champion_id)
        .chain(draft_state.bans.iter().map(|ban| ban.champion_id))
    {
        if let Some(champion) = champion_catalog.get(champion_id) {
            names.insert(champion_id, champion);
        }
    }

    names
}

pub fn status_message<'a>(status: &'a str, message: &'a str) -> StatusMessage<'a> {
    StatusMessage { status, message }
}

pub fn event_line<T>(event: &'static str, payload: &T) -> Result<String>
where
    T: Serialize,
{
    Ok(serde_json::to_string(&JsonEvent { event, payload })?)
}

pub fn print_event<T>(event: &'static str, payload: &T) -> Result<()>
where
    T: Serialize,
{
    println!("{}", event_line(event, payload)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;

    #[test]
    fn connection_event_hides_token() {
        let connection = LcuConnection {
            source: "test".to_string(),
            path: PathBuf::from("lockfile"),
            pid: Some(1234),
            port: 2999,
            password: "secret-token".to_string(),
            protocol: "https".to_string(),
        };

        let line = event_line("lcu_connection", &connection_summary(&connection)).unwrap();

        assert!(line.contains("\"event\":\"lcu_connection\""));
        assert!(line.contains("\"token_hidden\":true"));
        assert!(!line.contains("secret-token"));
    }

    #[test]
    fn summoner_event_hides_account_fields() {
        let summoner = json!({
            "profileIconId": 7079,
            "summonerLevel": 1160,
            "puuid": "should-not-render"
        });

        let line = event_line("summoner_summary", &summoner_summary(Some(&summoner))).unwrap();

        assert!(line.contains("\"profile_icon_id\":7079"));
        assert!(line.contains("\"account_fields_hidden\":true"));
        assert!(!line.contains("should-not-render"));
    }

    #[test]
    fn draft_event_includes_champion_names_for_rendering() {
        let catalog = super::super::champions::ChampionCatalog::from_lcu_summary(&json!([
            { "id": 103, "alias": "Ahri", "name": "阿狸" },
            { "id": 412, "alias": "Thresh", "name": "锤石" }
        ]));
        let draft_state = DraftState {
            connected: true,
            gameflow: "ChampSelect".to_string(),
            local_player_cell_id: None,
            my_team: vec![super::super::models::DraftPlayer {
                cell_id: 0,
                champion_id: Some(103),
                assigned_position: Some("middle".to_string()),
                summoner_id: None,
            }],
            their_team: vec![],
            bans: vec![super::super::models::DraftBan {
                champion_id: 412,
                team_id: Some(200),
            }],
        };
        let analysis = super::super::analysis::analyze_draft(&draft_state);

        let line = event_line(
            "draft_snapshot",
            &draft_snapshot("test", None, &draft_state, &analysis, &catalog, &[]),
        )
        .unwrap();

        assert!(line.contains("\"champion_names\""));
        assert!(line.contains("\"103\""));
        assert!(line.contains("阿狸"));
        assert!(line.contains("\"412\""));
        assert!(line.contains("锤石"));
    }
}
