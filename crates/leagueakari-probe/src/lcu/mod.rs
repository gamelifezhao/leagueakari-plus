mod auth;
mod champ_select;
mod client;
mod lockfile;
mod models;

use anyhow::Result;
use serde_json::Value;

pub async fn run_probe() -> Result<()> {
    let lockfile = lockfile::discover()?;

    println!("LCU lockfile found:");
    println!("  path: {}", lockfile.path.display());
    println!("  pid: {}", lockfile.pid);
    println!("  port: {}", lockfile.port);
    println!("  protocol: {}", lockfile.protocol);
    println!("  password: <hidden>");

    let client = client::LcuClient::new(&lockfile)?;

    let summoner: Value = client.get_json("/lol-summoner/v1/current-summoner").await?;
    print_json("current summoner", &summoner)?;

    let gameflow_phase: Value = client.get_json("/lol-gameflow/v1/gameflow-phase").await?;
    print_json("gameflow phase", &gameflow_phase)?;

    if let Some(gameflow) = gameflow_phase.as_str() {
        let empty_draft = models::DraftState::empty(gameflow.to_string());
        print_json("draft state", &serde_json::to_value(empty_draft)?)?;
    }

    if gameflow_phase.as_str() == Some("ChampSelect") {
        let session: Value = client.get_json("/lol-champ-select/v1/session").await?;
        print_json("champ select session", &session)?;

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

fn print_json(label: &str, value: &Value) -> Result<()> {
    println!();
    println!("{label}:");
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
