#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    ffi::OsString,
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

use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Listener, Manager};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const PROBE_EVENT: &str = "leagueakari-probe-event";
const FRONTEND_READY_EVENT: &str = "leagueakari-frontend-ready";
const PROBE_PATH_ENV: &str = "LEAGUEAKARI_PROBE_PATH";
const PROBE_RETRY_DELAY: Duration = Duration::from_secs(5);

#[derive(Clone, Default)]
struct ProbeProcess {
    child: Arc<Mutex<Option<Child>>>,
    started: Arc<AtomicBool>,
}

fn main() {
    tauri::Builder::default()
        .manage(ProbeProcess::default())
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

    if should_retry_probe(final_status) {
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

    PathBuf::from(executable_name)
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

fn should_retry_probe(final_status: &str) -> bool {
    final_status != "stopped"
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
    fn retries_probe_errors_but_not_clean_stops() {
        assert!(should_retry_probe("error"));
        assert!(should_retry_probe("parse_error"));
        assert!(!should_retry_probe("stopped"));
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
