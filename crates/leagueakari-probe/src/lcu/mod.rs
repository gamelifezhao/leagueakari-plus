mod analysis;
mod auth;
mod champ_select;
mod champions;
mod client;
mod connection;
mod models;
mod output;
mod websocket;

use anyhow::{Result, bail};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Default)]
pub struct ProbeOptions {
    pub raw: bool,
    pub watch: bool,
    pub json: bool,
    pub validate_data: bool,
}

pub async fn run_probe(options: ProbeOptions) -> Result<()> {
    if options.validate_data {
        return run_data_validation(options.json);
    }

    let LcuProbeConnection {
        connection,
        client,
        summoner,
        gameflow_phase,
    } = connect_to_lcu().await?;

    if options.json {
        output::print_event("lcu_connection", &output::connection_summary(&connection))?;
        output::print_event(
            "summoner_summary",
            &output::summoner_summary(summoner.as_ref()),
        )?;
        if options.raw {
            if let Some(summoner) = &summoner {
                output::print_event("raw_current_summoner", summoner)?;
            }
        }
    } else {
        print_connection_summary(&connection);
        if let Some(summoner) = &summoner {
            print_summoner_summary(summoner);
            if options.raw {
                print_json("current summoner", summoner)?;
            }
        } else {
            print_summoner_unavailable();
        }
    }

    if options.json {
        output::print_event("gameflow_phase", &output::gameflow_phase(&gameflow_phase))?;
    } else {
        print_json("gameflow phase", &gameflow_phase)?;
    }
    let champion_catalog = load_champion_catalog(&client).await;

    if let Some(gameflow) = gameflow_phase.as_str() {
        let empty_draft = models::DraftState::empty(gameflow.to_string());
        if options.json {
            let analysis = analysis::analyze_draft(&empty_draft);
            output::print_event(
                "draft_snapshot",
                &output::draft_snapshot(
                    "initial",
                    None,
                    &empty_draft,
                    &analysis,
                    &champion_catalog,
                ),
            )?;
        } else {
            print_json("draft state", &serde_json::to_value(empty_draft)?)?;
        }
    }

    if gameflow_phase.as_str() == Some("ChampSelect") {
        let session: Value = client.get_json("/lol-champ-select/v1/session").await?;
        let draft_state =
            champ_select::parse_draft_state(gameflow_phase.as_str().unwrap_or("Unknown"), &session);
        let analysis = analysis::analyze_draft(&draft_state);
        if options.json {
            if options.raw {
                output::print_event("raw_champ_select_session", &session)?;
            }
            output::print_event(
                "draft_snapshot",
                &output::draft_snapshot(
                    "initial",
                    None,
                    &draft_state,
                    &analysis,
                    &champion_catalog,
                ),
            )?;
        } else {
            print_champ_select_summary(&session);
            if options.raw {
                print_json("champ select session", &session)?;
            }

            print_draft_summary(&draft_state, &champion_catalog);
            print_json("composition analysis", &serde_json::to_value(analysis)?)?;
            print_json(
                "normalized draft state",
                &serde_json::to_value(draft_state)?,
            )?;
        }
    } else if options.json {
        output::print_event(
            "champ_select_status",
            &output::status_message("skipped", "gameflow is not ChampSelect"),
        )?;
    } else {
        println!("champ select session: skipped because gameflow is not ChampSelect");
    }

    if options.watch {
        websocket::watch(&connection, &champion_catalog, options.raw, options.json).await?;
    }

    if options.json {
        output::print_event("probe_status", &output::status_message("finished", ""))?;
    } else {
        println!("probe finished");
    }

    Ok(())
}

fn run_data_validation(json: bool) -> Result<()> {
    let report = analysis::validate_data();

    if json {
        output::print_event("data_validation", &report)?;
    } else {
        println!("data validation:");
        println!("  champion tags: {}", report.tag_count);
        println!("  OP.GG entries: {}", report.opgg_entry_count);
        println!("  OP.GG build entries: {}", report.opgg_build_count);
        println!("  matched OP.GG entries: {}", report.matched_opgg_entries);
        println!(
            "  matched OP.GG build entries: {}",
            report.matched_opgg_builds
        );
        println!(
            "  stat entries covered by builds: {}",
            report.build_covered_stat_entries
        );
        println!(
            "  stat entries covered by exact-context builds: {}",
            report.build_exact_context_stat_entries
        );
        println!(
            "  unmatched OP.GG entries: {}",
            format_list(report.unmatched_opgg_entries)
        );
        println!(
            "  unmatched OP.GG build entries: {}",
            format_list(report.unmatched_opgg_builds)
        );
        println!(
            "  missing OP.GG build stat entries: {}",
            format_list(report.missing_build_stat_entries)
        );
        println!(
            "  stale OP.GG build entries: {}",
            format_list(report.stale_build_entries)
        );
        println!(
            "  orphan OP.GG build entries: {}",
            format_list(report.orphan_build_entries)
        );
        println!(
            "  duplicate tag ids: {}",
            format_list(report.duplicate_tag_ids)
        );
        println!(
            "  duplicate tag keys: {}",
            format_list(report.duplicate_tag_keys)
        );
        println!(
            "  duplicate OP.GG stat keys: {}",
            format_list(report.duplicate_stat_keys)
        );
        println!(
            "  duplicate OP.GG build keys: {}",
            format_list(report.duplicate_build_keys)
        );
        println!(
            "  invalid OP.GG entries: {}",
            format_list(report.invalid_stat_entries)
        );
        println!(
            "  invalid OP.GG build entries: {}",
            format_list(report.invalid_build_entries)
        );
        println!("  warnings: {}", format_list(report.warnings));
    }

    Ok(())
}

async fn load_champion_catalog(client: &client::LcuClient) -> champions::ChampionCatalog {
    match client
        .get_json::<Value>("/lol-game-data/assets/v1/champion-summary.json")
        .await
    {
        Ok(value) => champions::ChampionCatalog::from_lcu_summary(&value),
        Err(error) => {
            tracing::debug!("failed to load champion summary: {error}");
            champions::ChampionCatalog::default()
        }
    }
}

struct LcuProbeConnection {
    connection: connection::LcuConnection,
    client: client::LcuClient,
    summoner: Option<Value>,
    gameflow_phase: Value,
}

async fn connect_to_lcu() -> Result<LcuProbeConnection> {
    let connections = connection::discover_all()?;
    let mut failures = Vec::new();

    for connection in connections {
        let client = client::LcuClient::new(&connection)?;
        match client
            .get_json::<Value>("/lol-gameflow/v1/gameflow-phase")
            .await
        {
            Ok(gameflow_phase) => {
                let summoner = match client
                    .get_json::<Value>("/lol-summoner/v1/current-summoner")
                    .await
                {
                    Ok(value) => Some(value),
                    Err(error) => {
                        tracing::debug!(
                            "LCU current summoner request skipped: source={}, port={}, error={}",
                            connection.source,
                            connection.port,
                            error
                        );
                        None
                    }
                };

                return Ok(LcuProbeConnection {
                    connection,
                    client,
                    summoner,
                    gameflow_phase,
                });
            }
            Err(error) => {
                tracing::debug!(
                    "LCU candidate failed: source={}, port={}, error={}",
                    connection.source,
                    connection.port,
                    error
                );
                failures.push(format!(
                    "{} port {}: {}",
                    connection.source, connection.port, error
                ));
            }
        }
    }

    if failures.is_empty() {
        Err(connection::LcuConnectionError::NotFound.into())
    } else {
        bail!(
            "all LCU connection candidates failed:\n  {}",
            failures.join("\n  ")
        );
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

fn print_summoner_unavailable() {
    println!();
    println!("current summoner summary: unavailable");
    println!("  account fields: hidden");
    println!("  note: continuing because gameflow is reachable");
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

fn print_draft_summary(
    draft_state: &models::DraftState,
    champion_catalog: &champions::ChampionCatalog,
) {
    println!();
    println!("draft summary:");
    println!(
        "  my picks: {}",
        format_players(&draft_state.my_team, champion_catalog)
    );
    println!(
        "  their picks: {}",
        format_players(&draft_state.their_team, champion_catalog)
    );
    println!(
        "  my bans: {}",
        format_bans(&draft_state.bans, Some(100), champion_catalog)
    );
    println!(
        "  their bans: {}",
        format_bans(&draft_state.bans, Some(200), champion_catalog)
    );
}

fn format_players(
    players: &[models::DraftPlayer],
    champion_catalog: &champions::ChampionCatalog,
) -> String {
    let labels = players
        .iter()
        .filter_map(|player| player.champion_id)
        .map(|champion_id| champion_catalog.label(champion_id))
        .collect::<Vec<_>>();

    format_list(labels)
}

fn format_bans(
    bans: &[models::DraftBan],
    team_id: Option<i64>,
    champion_catalog: &champions::ChampionCatalog,
) -> String {
    let labels = bans
        .iter()
        .filter(|ban| ban.team_id == team_id)
        .map(|ban| champion_catalog.label(ban.champion_id))
        .collect::<Vec<_>>();

    format_list(labels)
}

fn format_list<T>(items: Vec<T>) -> String
where
    T: ToString,
{
    if items.is_empty() {
        "none".to_string()
    } else {
        items
            .into_iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
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
