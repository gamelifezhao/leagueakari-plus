mod auth;
mod champ_select;
mod client;
mod connection;
mod models;

use anyhow::Result;
use serde_json::Value;

pub async fn run_probe(raw: bool) -> Result<()> {
    let (connection, client, summoner) = connect_to_lcu().await?;
    print_connection_summary(&connection);
    print_summoner_summary(&summoner);
    if raw {
        print_json("current summoner", &summoner)?;
    }

    let gameflow_phase: Value = client.get_json("/lol-gameflow/v1/gameflow-phase").await?;
    print_json("gameflow phase", &gameflow_phase)?;

    if let Some(gameflow) = gameflow_phase.as_str() {
        let empty_draft = models::DraftState::empty(gameflow.to_string());
        print_json("draft state", &serde_json::to_value(empty_draft)?)?;
    }

    if gameflow_phase.as_str() == Some("ChampSelect") {
        let session: Value = client.get_json("/lol-champ-select/v1/session").await?;
        print_champ_select_summary(&session);
        if raw {
            print_json("champ select session", &session)?;
        }

        let draft_state =
            champ_select::parse_draft_state(gameflow_phase.as_str().unwrap_or("Unknown"), &session);
        print_json(
            "normalized draft state",
            &serde_json::to_value(draft_state)?,
        )?;
    } else {
        println!("champ select session: skipped because gameflow is not ChampSelect");
    }

    println!("probe finished");

    Ok(())
}

async fn connect_to_lcu() -> Result<(connection::LcuConnection, client::LcuClient, Value)> {
    let connections = connection::discover_all()?;
    let mut last_error = None;

    for connection in connections {
        let client = client::LcuClient::new(&connection)?;
        match client
            .get_json::<Value>("/lol-summoner/v1/current-summoner")
            .await
        {
            Ok(summoner) => return Ok((connection, client, summoner)),
            Err(error) => {
                tracing::debug!(
                    "LCU candidate failed: source={}, port={}, error={}",
                    connection.source,
                    connection.port,
                    error
                );
                last_error = Some(error);
            }
        }
    }

    match last_error {
        Some(error) => Err(error.into()),
        None => Err(connection::LcuConnectionError::NotFound.into()),
    }
}

fn print_connection_summary(connection: &connection::LcuConnection) {
    println!("LCU connection found:");
    println!("  source: {}", connection.source);
    println!("  path: {}", connection.path.display());
    if let Some(pid) = connection.pid {
        println!("  pid: {pid}");
    }
    println!("  port: {}", connection.port);
    println!("  protocol: {}", connection.protocol);
    println!("  password/token: <hidden>");
}

fn print_summoner_summary(summoner: &Value) {
    println!();
    println!("current summoner summary:");
    println!(
        "  profile_icon_id: {}",
        display_value(summoner.get("profileIconId"))
    );
    println!(
        "  summoner_level: {}",
        display_value(summoner.get("summonerLevel"))
    );
    println!("  account fields: hidden");
}

fn print_champ_select_summary(session: &Value) {
    let local_player_cell_id = session
        .get("localPlayerCellId")
        .and_then(Value::as_i64)
        .map_or_else(|| "unknown".to_string(), |id| id.to_string());
    let my_team_count = session
        .get("myTeam")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let their_team_count = session
        .get("theirTeam")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let action_count = session
        .get("actions")
        .and_then(Value::as_array)
        .map(|groups| {
            groups
                .iter()
                .filter_map(Value::as_array)
                .map(Vec::len)
                .sum::<usize>()
        })
        .unwrap_or_default();

    println!();
    println!("champ select summary:");
    println!("  local_player_cell_id: {local_player_cell_id}");
    println!("  my_team_count: {my_team_count}");
    println!("  their_team_count: {their_team_count}");
    println!("  action_count: {action_count}");
}

fn print_json(label: &str, value: &Value) -> Result<()> {
    println!();
    println!("{label}:");
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn display_value(value: Option<&Value>) -> String {
    value.map_or_else(|| "unknown".to_string(), ToString::to_string)
}
