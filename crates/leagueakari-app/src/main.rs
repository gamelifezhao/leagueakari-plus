#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    ffi::OsString,
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Listener, Manager};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const PROBE_EVENT: &str = "leagueakari-probe-event";
const FRONTEND_READY_EVENT: &str = "leagueakari-frontend-ready";
const PROBE_PATH_ENV: &str = "LEAGUEAKARI_PROBE_PATH";
const PROBE_RETRY_DELAY: Duration = Duration::from_secs(5);

include!(concat!(env!("OUT_DIR"), "/embedded_probe.rs"));

#[derive(Clone, Default)]
struct ProbeProcess {
    child: Arc<Mutex<Option<Child>>>,
    started: Arc<AtomicBool>,
}

fn main() {
    tauri::Builder::default()
        .manage(ProbeProcess::default())
        .invoke_handler(tauri::generate_handler![
            fetch_recent_matches,
            fetch_opgg_champion
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();
            let probe_process = app.state::<ProbeProcess>().inner().clone();
            let bridge_handle = app_handle.clone();

            app_handle.listen(FRONTEND_READY_EVENT, move |_| {
                start_probe_bridge(bridge_handle.clone(), probe_process.clone());
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                let probe_process = window.state::<ProbeProcess>();
                stop_probe_bridge(probe_process.inner());
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to run LeagueAkari Plus desktop app");
}

#[tauri::command]
async fn fetch_recent_matches() -> Result<String, String> {
    let probe_path = resolve_probe_path();
    tauri::async_runtime::spawn_blocking(move || run_recent_matches_probe(&probe_path))
        .await
        .map_err(|error| format!("recent matches task failed: {error}"))?
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpggChampionRequest {
    champion_id: i64,
    role: String,
    #[serde(default = "default_opgg_region")]
    region: String,
    #[serde(default = "default_opgg_mode")]
    mode: String,
    #[serde(default = "default_opgg_tier")]
    tier: String,
    version: Option<String>,
}

#[derive(Debug, Serialize)]
struct OpggBuildResponse {
    source: &'static str,
    patch: String,
    region: String,
    tier: String,
    queue: String,
    champion_id: i64,
    champion_name: String,
    role: String,
    win_rate: Option<f64>,
    pick_rate: Option<f64>,
    ban_rate: Option<f64>,
    rank: Option<i64>,
    sample_count: Option<u64>,
    summoner_spells: Vec<OpggRateBuild<i64>>,
    runes: Vec<OpggRuneBuild>,
    skill_order: Option<OpggSkillOrder>,
    starter_items: Vec<OpggRateBuild<i64>>,
    boots: Vec<OpggRateBuild<i64>>,
    core_items: Vec<OpggRateBuild<i64>>,
    last_items: Vec<OpggRateBuild<i64>>,
}

#[derive(Debug, Serialize)]
struct OpggRateBuild<T> {
    ids: Vec<T>,
    pick_rate: Option<f64>,
    win_rate: Option<f64>,
    games: Option<u64>,
}

#[derive(Debug, Serialize)]
struct OpggRuneBuild {
    primary_style_id: i64,
    secondary_style_id: i64,
    perk_ids: Vec<i64>,
    stat_mod_ids: Vec<i64>,
    pick_rate: Option<f64>,
    win_rate: Option<f64>,
    games: Option<u64>,
}

#[derive(Debug, Serialize)]
struct OpggSkillOrder {
    priority: Vec<String>,
    order: Vec<String>,
    pick_rate: Option<f64>,
    win_rate: Option<f64>,
    games: Option<u64>,
}

fn default_opgg_region() -> String {
    "global".to_string()
}

fn default_opgg_mode() -> String {
    "ranked".to_string()
}

fn default_opgg_tier() -> String {
    "emerald_plus".to_string()
}

#[tauri::command]
async fn fetch_opgg_champion(request: OpggChampionRequest) -> Result<OpggBuildResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) LeagueAkariPlus/0.1")
        .build()
        .map_err(|error| format!("failed to build OP.GG client: {error}"))?;

    let version = match request.version {
        Some(version) if !version.trim().is_empty() => version,
        _ => fetch_latest_opgg_version(&client, &request.region, &request.mode).await?,
    };

    let role = normalize_opgg_role(&request.role);
    let url = format!(
        "https://lol-api-champion.op.gg/api/{}/champions/{}/{}/{}",
        request.region, request.mode, request.champion_id, role
    );

    let payload = client
        .get(&url)
        .query(&[
            ("tier", request.tier.as_str()),
            ("version", version.as_str()),
        ])
        .send()
        .await
        .map_err(|error| format!("OP.GG request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("OP.GG returned an error: {error}"))?
        .json::<Value>()
        .await
        .map_err(|error| format!("failed to parse OP.GG response: {error}"))?;

    Ok(normalize_opgg_response(
        &payload,
        request.champion_id,
        &role,
        &request.region,
        &request.tier,
        &request.mode,
        &version,
    ))
}

async fn fetch_latest_opgg_version(
    client: &reqwest::Client,
    region: &str,
    mode: &str,
) -> Result<String, String> {
    let url = format!("https://lol-api-champion.op.gg/api/{region}/champions/{mode}/versions");
    let payload = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("OP.GG version request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("OP.GG version returned an error: {error}"))?
        .json::<Value>()
        .await
        .map_err(|error| format!("failed to parse OP.GG versions: {error}"))?;

    payload
        .get("data")
        .and_then(Value::as_array)
        .and_then(|versions| versions.first())
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| "OP.GG did not return a version".to_string())
}

fn normalize_opgg_response(
    payload: &Value,
    champion_id: i64,
    role: &str,
    region: &str,
    tier: &str,
    mode: &str,
    version: &str,
) -> OpggBuildResponse {
    let data = payload.get("data").unwrap_or(payload);
    let summary = data.get("summary").unwrap_or(&Value::Null);
    let average_stats = summary.get("average_stats").unwrap_or(&Value::Null);

    OpggBuildResponse {
        source: "OP.GG public champion API",
        patch: version.to_string(),
        region: region.to_string(),
        tier: tier.to_string(),
        queue: mode.to_string(),
        champion_id,
        champion_name: summary
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        role: role.to_string(),
        win_rate: percent_from_ratio(average_stats.get("win_rate")),
        pick_rate: percent_from_ratio(average_stats.get("pick_rate")),
        ban_rate: percent_from_ratio(average_stats.get("ban_rate")),
        rank: average_stats
            .get("tier_data")
            .and_then(|value| value.get("rank"))
            .and_then(Value::as_i64)
            .or_else(|| average_stats.get("rank").and_then(Value::as_i64)),
        sample_count: average_stats.get("play").and_then(Value::as_u64),
        summoner_spells: rate_builds(data.get("summoner_spells"), 2),
        runes: rune_builds(data.get("runes"), 2),
        skill_order: skill_order(data.get("skill_masteries")),
        starter_items: rate_builds(data.get("starter_items"), 2),
        boots: rate_builds(data.get("boots"), 2),
        core_items: rate_builds(data.get("core_items"), 2),
        last_items: rate_builds(data.get("last_items"), 2),
    }
}

fn normalize_opgg_role(role: &str) -> String {
    match role.to_ascii_lowercase().as_str() {
        "bottom" => "adc",
        "middle" => "mid",
        "utility" => "support",
        "none" => "none",
        other => other,
    }
    .to_string()
}

fn rate_builds(value: Option<&Value>, limit: usize) -> Vec<OpggRateBuild<i64>> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(limit)
        .map(|item| OpggRateBuild {
            ids: item
                .get("ids")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_i64)
                .collect(),
            pick_rate: percent_from_ratio(item.get("pick_rate")),
            win_rate: win_rate_from_counts(item),
            games: item.get("play").and_then(Value::as_u64),
        })
        .collect()
}

fn rune_builds(value: Option<&Value>, limit: usize) -> Vec<OpggRuneBuild> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(limit)
        .map(|rune| {
            let mut perk_ids = Vec::new();
            perk_ids.extend(int_array(rune.get("primary_rune_ids")));
            perk_ids.extend(int_array(rune.get("secondary_rune_ids")));

            OpggRuneBuild {
                primary_style_id: rune
                    .get("primary_page_id")
                    .and_then(Value::as_i64)
                    .unwrap_or_default(),
                secondary_style_id: rune
                    .get("secondary_page_id")
                    .and_then(Value::as_i64)
                    .unwrap_or_default(),
                perk_ids,
                stat_mod_ids: int_array(rune.get("stat_mod_ids")),
                pick_rate: percent_from_ratio(rune.get("pick_rate")),
                win_rate: win_rate_from_counts(rune),
                games: rune.get("play").and_then(Value::as_u64),
            }
        })
        .collect()
}

fn skill_order(value: Option<&Value>) -> Option<OpggSkillOrder> {
    let mastery = value.and_then(Value::as_array)?.first()?;
    let build = mastery
        .get("builds")
        .and_then(Value::as_array)
        .and_then(|builds| builds.first());

    Some(OpggSkillOrder {
        priority: mastery
            .get("ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect(),
        order: build
            .and_then(|value| value.get("order"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect(),
        pick_rate: percent_from_ratio(mastery.get("pick_rate")),
        win_rate: win_rate_from_counts(mastery),
        games: mastery.get("play").and_then(Value::as_u64),
    })
}

fn int_array(value: Option<&Value>) -> Vec<i64> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .collect()
}

fn percent_from_ratio(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64).map(|number| number * 100.0)
}

fn win_rate_from_counts(value: &Value) -> Option<f64> {
    let win = value.get("win").and_then(Value::as_f64)?;
    let play = value.get("play").and_then(Value::as_f64)?;
    if play <= 0.0 {
        return None;
    }
    Some((win / play) * 100.0)
}

fn run_recent_matches_probe(probe_path: &PathBuf) -> Result<String, String> {
    let mut command = Command::new(probe_path);
    command
        .args(["--recent-matches", "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let output = command
        .output()
        .map_err(|error| format!("failed to start recent matches probe: {error}"))?;
    if output.status.success() {
        return String::from_utf8(output.stdout)
            .map_err(|error| format!("recent matches probe returned invalid UTF-8: {error}"));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let message = if !stderr.trim().is_empty() {
        stderr.trim().to_string()
    } else if !stdout.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        format!("recent matches probe exited with {}", output.status)
    };

    Err(message)
}

fn start_probe_bridge(app: AppHandle, probe_process: ProbeProcess) {
    if probe_process.started.swap(true, Ordering::SeqCst) {
        emit_bridge_status(&app, "already_running", "probe bridge is already running");
        return;
    }

    thread::spawn(move || {
        let probe_path = resolve_probe_path();
        let mut attempt = 1_u64;

        while probe_process.started.load(Ordering::SeqCst) {
            let outcome = run_probe_once(&app, &probe_process, &probe_path, attempt);

            if !probe_process.started.load(Ordering::SeqCst) {
                break;
            }

            match outcome {
                ProbeRunOutcome::ListeningEnded => {
                    probe_process.started.store(false, Ordering::SeqCst);
                    break;
                }
                ProbeRunOutcome::Failed(message) => {
                    attempt += 1;
                    emit_bridge_status(
                        &app,
                        "retrying",
                        &format!(
                            "{message}; retrying in {} seconds",
                            PROBE_RETRY_DELAY.as_secs()
                        ),
                    );
                    thread::sleep(PROBE_RETRY_DELAY);
                }
            }
        }
    });
}

enum ProbeRunOutcome {
    ListeningEnded,
    Failed(String),
}

fn run_probe_once(
    app: &AppHandle,
    probe_process: &ProbeProcess,
    probe_path: &PathBuf,
    attempt: u64,
) -> ProbeRunOutcome {
    let last_stderr = Arc::new(Mutex::new(None::<String>));
    emit_bridge_status(
        app,
        "starting",
        &format!("starting {} (attempt {attempt})", probe_path.display()),
    );

    let mut command = Command::new(probe_path);
    command
        .args(["--watch", "--json"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return ProbeRunOutcome::Failed(format!("failed to start probe: {error}"));
        }
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let pid = child.id();

    {
        let mut child_slot = probe_process
            .child
            .lock()
            .expect("probe child process lock poisoned");
        *child_slot = Some(child);
    }

    emit_bridge_status(app, "running", &format!("probe process started: {pid}"));

    let stderr_reader = if let Some(stderr) = stderr {
        let stderr_app = app.clone();
        let stderr_state = last_stderr.clone();
        Some(thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                let line = line.trim();
                if !line.is_empty() {
                    let mut last_stderr = stderr_state
                        .lock()
                        .expect("probe stderr status lock poisoned");
                    *last_stderr = Some(line.to_string());
                    emit_bridge_status(&stderr_app, "stderr", line);
                }
            }
        }))
    } else {
        None
    };

    if let Some(stdout) = stdout {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<Value>(line) {
                Ok(event) => {
                    let _ = app.emit(PROBE_EVENT, event);
                }
                Err(error) => emit_bridge_status(
                    app,
                    "parse_error",
                    &format!("failed to parse probe JSON event: {error}"),
                ),
            }
        }
    }

    let exit_status = {
        let mut child_slot = probe_process
            .child
            .lock()
            .expect("probe child process lock poisoned");
        child_slot.take().and_then(|mut child| child.wait().ok())
    };
    let stop_requested = !probe_process.started.load(Ordering::SeqCst);

    if let Some(stderr_reader) = stderr_reader {
        let _ = stderr_reader.join();
    }

    let last_stderr = last_stderr
        .lock()
        .expect("probe stderr status lock poisoned")
        .clone();
    let (final_status, exit_message) =
        format_probe_exit_status(exit_status, last_stderr.as_deref());

    emit_bridge_status(app, final_status, &exit_message);

    if should_retry_probe(final_status, stop_requested) {
        ProbeRunOutcome::Failed(exit_message)
    } else {
        ProbeRunOutcome::ListeningEnded
    }
}

fn stop_probe_bridge(probe_process: &ProbeProcess) {
    let mut child_slot = probe_process
        .child
        .lock()
        .expect("probe child process lock poisoned");

    if let Some(child) = child_slot.as_mut() {
        let _ = child.kill();
    }
    *child_slot = None;
    probe_process.started.store(false, Ordering::SeqCst);
}

fn resolve_probe_path() -> PathBuf {
    resolve_probe_path_from(
        std::env::var_os(PROBE_PATH_ENV),
        std::env::current_exe().ok(),
        probe_executable_name(),
    )
}

fn probe_executable_name() -> &'static str {
    if cfg!(windows) {
        "leagueakari-probe.exe"
    } else {
        "leagueakari-probe"
    }
}

fn resolve_probe_path_from(
    env_override: Option<OsString>,
    current_exe: Option<PathBuf>,
    executable_name: &str,
) -> PathBuf {
    if let Some(env_override) = env_override {
        if !env_override.as_os_str().is_empty() {
            return PathBuf::from(env_override);
        }
    }

    if let Some(current_exe) = current_exe {
        if let Some(directory) = current_exe.parent() {
            let sibling = directory.join(executable_name);
            if sibling.is_file() {
                return sibling;
            }
        }
    }

    if let Some(extracted_probe) = extract_embedded_probe(executable_name) {
        return extracted_probe;
    }

    PathBuf::from(executable_name)
}

fn extract_embedded_probe(executable_name: &str) -> Option<PathBuf> {
    let bytes = EMBEDDED_PROBE_BYTES?;
    let directory = std::env::temp_dir().join("LeagueAkari Plus");
    let path = directory.join(executable_name);

    if fs::create_dir_all(&directory).is_err() {
        return None;
    }
    if fs::write(&path, bytes).is_err() {
        return None;
    }

    Some(path)
}

fn format_probe_exit_status(
    exit_status: Option<std::process::ExitStatus>,
    last_stderr: Option<&str>,
) -> (&'static str, String) {
    match (exit_status, last_stderr) {
        (Some(status), Some(stderr)) if !status.success() => {
            ("error", format!("{stderr} ({status})"))
        }
        (Some(status), _) if status.success() => {
            ("stopped", format!("probe process exited: {status}"))
        }
        (Some(status), _) => ("error", format!("probe process exited: {status}")),
        (None, Some(stderr)) => ("error", stderr.to_string()),
        (None, None) => ("stopped", "probe process stopped".to_string()),
    }
}

fn should_retry_probe(final_status: &str, stop_requested: bool) -> bool {
    !stop_requested && (final_status == "stopped" || final_status == "error")
}

fn emit_bridge_status(app: &AppHandle, status: &str, message: &str) {
    let _ = app.emit(
        PROBE_EVENT,
        json!({
            "event": "probe_bridge_status",
            "payload": {
                "status": status,
                "message": message
            }
        }),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    #[test]
    fn env_override_wins_over_other_probe_paths() {
        let override_path = PathBuf::from(r"C:\LeagueAkari\custom-probe.exe");

        let resolved = resolve_probe_path_from(
            Some(override_path.clone().into_os_string()),
            None,
            "leagueakari-probe.exe",
        );

        assert_eq!(resolved, override_path);
    }

    #[test]
    fn uses_probe_next_to_current_executable_when_present() {
        let directory = unique_temp_dir();
        fs::create_dir_all(&directory).expect("create temp test directory");
        let app_path = directory.join("leagueakari-app.exe");
        let probe_path = directory.join("leagueakari-probe.exe");
        fs::write(&app_path, b"app").expect("write app test file");
        fs::write(&probe_path, b"probe").expect("write probe test file");

        let resolved = resolve_probe_path_from(None, Some(app_path), "leagueakari-probe.exe");

        assert_eq!(resolved, probe_path);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn falls_back_to_executable_name_when_no_sibling_exists() {
        let directory = unique_temp_dir();
        fs::create_dir_all(&directory).expect("create temp test directory");
        let app_path = directory.join("leagueakari-app.exe");
        fs::write(&app_path, b"app").expect("write app test file");

        let resolved = resolve_probe_path_from(None, Some(app_path), "leagueakari-probe.exe");

        assert_eq!(resolved, PathBuf::from("leagueakari-probe.exe"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn retries_probe_exit_until_stop_is_requested() {
        assert!(should_retry_probe("error", false));
        assert!(should_retry_probe("stopped", false));
        assert!(!should_retry_probe("parse_error", false));
        assert!(!should_retry_probe("stopped", true));
        assert!(!should_retry_probe("error", true));
    }

    fn unique_temp_dir() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "leagueakari-app-test-{}-{suffix}",
            std::process::id()
        ))
    }
}
