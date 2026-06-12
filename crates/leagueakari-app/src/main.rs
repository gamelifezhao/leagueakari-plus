#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, Listener, Manager};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const PROBE_EVENT: &str = "leagueakari-probe-event";
const FRONTEND_READY_EVENT: &str = "leagueakari-frontend-ready";

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
        emit_bridge_status(
            &app,
            "starting",
            &format!("starting {}", probe_path.display()),
        );

        let mut command = Command::new(&probe_path);
        command
            .args(["--watch", "--json"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);

        let child_result = command.spawn();

        let mut child = match child_result {
            Ok(child) => child,
            Err(error) => {
                probe_process.started.store(false, Ordering::SeqCst);
                emit_bridge_status(&app, "error", &format!("failed to start probe: {error}"));
                return;
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

        emit_bridge_status(&app, "running", &format!("probe process started: {pid}"));

        if let Some(stderr) = stderr {
            let stderr_app = app.clone();
            thread::spawn(move || {
                for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                    if !line.trim().is_empty() {
                        emit_bridge_status(&stderr_app, "stderr", &line);
                    }
                }
            });
        }

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
                        &app,
                        "parse_error",
                        &format!("failed to parse probe JSON event: {error}"),
                    ),
                }
            }
        }

        let exit_message = {
            let mut child_slot = probe_process
                .child
                .lock()
                .expect("probe child process lock poisoned");
            child_slot
                .take()
                .and_then(|mut child| child.wait().ok())
                .map(|status| format!("probe process exited: {status}"))
                .unwrap_or_else(|| "probe process stopped".to_string())
        };

        probe_process.started.store(false, Ordering::SeqCst);
        emit_bridge_status(&app, "stopped", &exit_message);
    });
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
    let executable_name = if cfg!(windows) {
        "leagueakari-probe.exe"
    } else {
        "leagueakari-probe"
    };

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(directory) = current_exe.parent() {
            let sibling = directory.join(executable_name);
            if sibling.is_file() {
                return sibling;
            }
        }
    }

    PathBuf::from(executable_name)
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
