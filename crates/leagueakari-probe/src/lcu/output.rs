use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use super::{analysis::CompositionAnalysis, connection::LcuConnection, models::DraftState};

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
) -> DraftSnapshot<'a> {
    DraftSnapshot {
        source,
        lcu_event_type,
        draft_state,
        analysis,
    }
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
}
