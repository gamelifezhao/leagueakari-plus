use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use thiserror::Error;

const MAX_LOG_FILES: usize = 12;
const MAX_LOG_CONNECTIONS: usize = 24;

#[derive(Debug, Clone)]
pub struct LcuConnection {
    pub source: String,
    pub path: PathBuf,
    pub pid: Option<u32>,
    pub port: u16,
    pub password: String,
    pub protocol: String,
}

#[derive(Debug, Error)]
pub enum LcuConnectionError {
    #[error("LCU connection was not found. Start the League client, then run the probe again.")]
    NotFound,
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(
        "invalid standard lockfile format at {path}: expected LeagueClient:pid:port:password:protocol"
    )]
    InvalidLockfileFormat { path: PathBuf },
    #[error("invalid pid in standard lockfile at {path}: {value}")]
    InvalidPid { path: PathBuf, value: String },
    #[error("invalid port in LCU connection source at {path}: {value}")]
    InvalidPort { path: PathBuf, value: String },
}

pub fn discover_all() -> Result<Vec<LcuConnection>, LcuConnectionError> {
    let mut connections = Vec::new();

    connections.extend(discover_from_processes()?);
    connections.extend(discover_standard_lockfiles()?);
    connections.extend(discover_from_logs()?);
    dedupe_connections(&mut connections);

    if connections.is_empty() {
        Err(LcuConnectionError::NotFound)
    } else {
        Ok(connections)
    }
}

fn discover_from_processes() -> Result<Vec<LcuConnection>, LcuConnectionError> {
    #[cfg(windows)]
    {
        discover_from_windows_processes()
    }

    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}

#[cfg(windows)]
fn discover_from_windows_processes() -> Result<Vec<LcuConnection>, LcuConnectionError> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-CimInstance Win32_Process -Filter \"Name='LeagueClientUx.exe'\" | Select-Object ProcessId,ExecutablePath,CommandLine | ConvertTo-Json -Compress",
        ])
        .output()
        .map_err(|source| LcuConnectionError::Read {
            path: PathBuf::from("Win32_Process:LeagueClientUx.exe"),
            source,
        })?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_process_json(&stdout)
}

#[cfg(windows)]
fn parse_process_json(content: &str) -> Result<Vec<LcuConnection>, LcuConnectionError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let value = match serde_json::from_str::<serde_json::Value>(trimmed) {
        Ok(value) => value,
        Err(error) => {
            tracing::debug!("failed to parse LeagueClientUx process json: {error}");
            return Ok(Vec::new());
        }
    };

    let processes: Vec<serde_json::Value> = if let Some(items) = value.as_array() {
        items.clone()
    } else {
        vec![value]
    };

    let mut connections = Vec::new();
    for process in processes {
        let command_line = process
            .get("CommandLine")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let executable = process
            .get("ExecutablePath")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let pid = process
            .get("ProcessId")
            .and_then(serde_json::Value::as_u64)
            .and_then(|pid| u32::try_from(pid).ok());
        if let Some(connection) = parse_process_connection(command_line, executable, pid) {
            connections.push(connection);
        }
    }

    Ok(connections)
}

fn parse_process_connection(
    command_line: &str,
    executable: &str,
    pid: Option<u32>,
) -> Option<LcuConnection> {
    let port = find_flag_values(command_line, "--app-port=")
        .last()
        .and_then(|port| parse_port(Path::new("LeagueClientUx.exe"), port).ok())?;
    let token = find_flag_values(command_line, "--remoting-auth-token=")
        .last()
        .map(|token| (*token).to_string())?;
    let path = if executable.is_empty() {
        PathBuf::from("LeagueClientUx.exe")
    } else {
        PathBuf::from(executable)
    };

    Some(LcuConnection {
        source: process_argument_source(&path),
        path,
        pid,
        port,
        password: token,
        protocol: "https".to_string(),
    })
}

fn discover_standard_lockfiles() -> Result<Vec<LcuConnection>, LcuConnectionError> {
    let mut connections = Vec::new();

    for path in standard_lockfile_paths().into_iter().filter(|path| {
        path.is_file()
            && path
                .metadata()
                .map(|metadata| metadata.len() > 0)
                .unwrap_or(false)
    }) {
        match read_standard_lockfile(path.clone()) {
            Ok(connection) => connections.push(connection),
            Err(error) => {
                tracing::debug!("skipping invalid LCU lockfile {}: {error}", path.display())
            }
        }
    }

    Ok(connections)
}

fn read_standard_lockfile(path: PathBuf) -> Result<LcuConnection, LcuConnectionError> {
    let content = read_to_string(&path)?;
    parse_standard_lockfile(&path, content.trim())
}

fn parse_standard_lockfile(
    path: &Path,
    content: &str,
) -> Result<LcuConnection, LcuConnectionError> {
    let parts: Vec<&str> = content.split(':').collect();
    if parts.len() != 5 || parts[0] != "LeagueClient" {
        return Err(LcuConnectionError::InvalidLockfileFormat {
            path: path.to_path_buf(),
        });
    }

    let pid = parts[1]
        .parse::<u32>()
        .map_err(|_| LcuConnectionError::InvalidPid {
            path: path.to_path_buf(),
            value: parts[1].to_string(),
        })?;
    let port = parse_port(path, parts[2])?;

    Ok(LcuConnection {
        source: lockfile_source(path),
        path: path.to_path_buf(),
        pid: Some(pid),
        port,
        password: parts[3].to_string(),
        protocol: parts[4].to_string(),
    })
}

fn discover_from_logs() -> Result<Vec<LcuConnection>, LcuConnectionError> {
    let mut connections = Vec::new();

    for path in latest_log_paths().into_iter().take(MAX_LOG_FILES) {
        match read_log_lossy(&path).and_then(|content| parse_log_connections(&path, &content)) {
            Ok(found) => connections.extend(found),
            Err(error) => {
                tracing::debug!("skipping unreadable LCU log {}: {error}", path.display())
            }
        }
    }

    Ok(connections.into_iter().take(MAX_LOG_CONNECTIONS).collect())
}

fn parse_log_connections(
    path: &Path,
    content: &str,
) -> Result<Vec<LcuConnection>, LcuConnectionError> {
    let ports = find_flag_values(content, "--app-port=");
    let tokens = find_flag_values(content, "--remoting-auth-token=");
    let pair_count = ports.len().min(tokens.len());
    let start = ports.len().saturating_sub(pair_count);
    let token_start = tokens.len().saturating_sub(pair_count);
    let mut connections = Vec::new();

    for (port, token) in ports[start..]
        .iter()
        .zip(tokens[token_start..].iter())
        .rev()
    {
        connections.push(LcuConnection {
            source: log_argument_source(path),
            path: path.to_path_buf(),
            pid: None,
            port: parse_port(path, port)?,
            password: (*token).to_string(),
            protocol: "https".to_string(),
        });
    }

    Ok(connections)
}

fn find_flag_values<'a>(content: &'a str, flag: &str) -> Vec<&'a str> {
    content
        .match_indices(flag)
        .filter_map(|(index, _)| content[index + flag.len()..].split_whitespace().next())
        .map(|value| value.trim_matches('"'))
        .filter(|value| !value.is_empty())
        .collect()
}

fn standard_lockfile_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(path) = env::var("LEAGUEAKARI_LOCKFILE") {
        paths.push(PathBuf::from(path));
    }

    if let Ok(cwd) = env::current_dir() {
        paths.push(cwd.join("lockfile"));
    }

    for base in [
        env::var_os("LOCALAPPDATA").map(PathBuf::from),
        env::var_os("PROGRAMDATA").map(PathBuf::from),
    ]
    .into_iter()
    .flatten()
    {
        paths.push(
            base.join("Riot Games")
                .join("League of Legends")
                .join("lockfile"),
        );
    }

    for path in common_league_roots() {
        paths.push(path.join("LeagueClient").join("lockfile"));
        paths.push(path.join("lockfile"));
    }

    paths
}

fn latest_log_paths() -> Vec<PathBuf> {
    let mut files = Vec::new();

    for root in common_league_roots() {
        let league_client = root.join("LeagueClient");
        if let Ok(entries) = fs::read_dir(&league_client) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };

                if is_league_client_ux_log(file_name) {
                    let modified = entry
                        .metadata()
                        .and_then(|metadata| metadata.modified())
                        .ok();
                    files.push((modified, path));
                }
            }
        }
    }

    files.sort_by(|left, right| right.0.cmp(&left.0));
    files.into_iter().map(|(_, path)| path).collect()
}

fn common_league_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from(r"C:\Riot Games\League of Legends"),
        PathBuf::from(r"C:\Program Files\Riot Games\League of Legends"),
        PathBuf::from(r"C:\Program Files (x86)\Riot Games\League of Legends"),
        PathBuf::from(r"D:\Riot Games\League of Legends"),
        PathBuf::from(r"E:\Riot Games\League of Legends"),
        PathBuf::from(r"F:\Riot Games\League of Legends"),
        PathBuf::from(r"D:\WeGameApps\英雄联盟"),
        PathBuf::from(r"E:\WeGameApps\英雄联盟"),
        PathBuf::from(r"F:\WeGameApps\英雄联盟"),
    ];

    if let Ok(path) = env::var("LEAGUEAKARI_LEAGUE_ROOT") {
        roots.insert(0, PathBuf::from(path));
    }

    if let Ok(path) = env::var("LEAGUEAKARI_WEGAME_ROOT") {
        roots.insert(0, PathBuf::from(path));
    }

    for letter in b'C'..=b'Z' {
        roots.push(PathBuf::from(format!(
            r"{}:\WeGameApps\英雄联盟",
            letter as char
        )));
    }

    dedupe_paths(&mut roots);
    roots
}

fn lockfile_source(path: &Path) -> String {
    if is_wegame_path(path) {
        "WeGame standard lockfile".to_string()
    } else {
        "standard lockfile".to_string()
    }
}

fn log_argument_source(path: &Path) -> String {
    if is_wegame_path(path) {
        "WeGame LeagueClientUx log arguments".to_string()
    } else {
        "LeagueClientUx log arguments".to_string()
    }
}

fn process_argument_source(path: &Path) -> String {
    if is_wegame_path(path) {
        "WeGame LeagueClientUx process arguments".to_string()
    } else {
        "LeagueClientUx process arguments".to_string()
    }
}

fn is_wegame_path(path: &Path) -> bool {
    path.to_string_lossy()
        .to_ascii_lowercase()
        .contains("wegameapps")
}

fn is_league_client_ux_log(file_name: &str) -> bool {
    file_name.ends_with("LeagueClientUx.log") && !file_name.contains("LeagueClientUxHelper")
}

fn dedupe_paths(paths: &mut Vec<PathBuf>) {
    let mut seen = Vec::new();
    paths.retain(|path| {
        let key = path.to_string_lossy().to_ascii_lowercase();
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
}

fn read_to_string(path: &Path) -> Result<String, LcuConnectionError> {
    fs::read_to_string(path).map_err(|source| LcuConnectionError::Read {
        path: path.to_path_buf(),
        source,
    })
}

fn read_log_lossy(path: &Path) -> Result<String, LcuConnectionError> {
    fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(|source| LcuConnectionError::Read {
            path: path.to_path_buf(),
            source,
        })
}

fn parse_port(path: &Path, value: &str) -> Result<u16, LcuConnectionError> {
    value
        .parse::<u16>()
        .map_err(|_| LcuConnectionError::InvalidPort {
            path: path.to_path_buf(),
            value: value.to_string(),
        })
}

fn dedupe_connections(connections: &mut Vec<LcuConnection>) {
    let mut seen = Vec::new();
    connections.retain(|connection| {
        let key = (connection.port, connection.password.clone());
        if seen.contains(&key) {
            false
        } else {
            seen.push(key);
            true
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_lockfile() {
        let connection =
            parse_standard_lockfile(Path::new("lockfile"), "LeagueClient:1234:5678:secret:https")
                .unwrap();

        assert_eq!(connection.pid, Some(1234));
        assert_eq!(connection.port, 5678);
        assert_eq!(connection.password, "secret");
        assert_eq!(connection.protocol, "https");
    }

    #[test]
    fn parses_log_arguments() {
        let content = r#"Command line arguments: --remoting-auth-token=token-value --app-port=58425 --install-directory=test"#;
        let connection = parse_log_connections(Path::new("LeagueClientUx.log"), content)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        assert_eq!(connection.port, 58425);
        assert_eq!(connection.password, "token-value");
        assert_eq!(connection.protocol, "https");
    }

    #[test]
    fn uses_last_log_arguments() {
        let content = "--remoting-auth-token=old --app-port=11111\n--remoting-auth-token=new --app-port=22222";
        let connection = parse_log_connections(Path::new("LeagueClientUx.log"), content)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        assert_eq!(connection.port, 22222);
        assert_eq!(connection.password, "new");
    }

    #[test]
    fn parses_multiple_log_argument_pairs_newest_first() {
        let content = "--remoting-auth-token=old --app-port=11111\n--remoting-auth-token=new --app-port=22222";
        let connections = parse_log_connections(Path::new("LeagueClientUx.log"), content).unwrap();

        assert_eq!(connections.len(), 2);
        assert_eq!(connections[0].port, 22222);
        assert_eq!(connections[1].port, 11111);
    }

    #[test]
    fn labels_wegame_log_arguments() {
        let content = "--remoting-auth-token=token --app-port=33333";
        let connection = parse_log_connections(
            Path::new(r"F:\WeGameApps\英雄联盟\LeagueClient\LeagueClientUx.log"),
            content,
        )
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

        assert_eq!(connection.source, "WeGame LeagueClientUx log arguments");
    }

    #[test]
    fn parses_process_arguments() {
        let connection = parse_process_connection(
            r#""F:\WeGameApps\英雄联盟\LeagueClient\LeagueClientUx.exe" --remoting-auth-token=token --app-port=44444"#,
            r"F:\WeGameApps\英雄联盟\LeagueClient\LeagueClientUx.exe",
            Some(1234),
        )
        .unwrap();

        assert_eq!(connection.source, "WeGame LeagueClientUx process arguments");
        assert_eq!(connection.pid, Some(1234));
        assert_eq!(connection.port, 44444);
        assert_eq!(connection.password, "token");
    }

    #[test]
    fn ignores_league_client_ux_helper_logs() {
        assert!(is_league_client_ux_log(
            "2026-06-27T17-14-42_49636_73220_LeagueClientUx.log"
        ));
        assert!(!is_league_client_ux_log(
            "2026-06-27T17-14-42_49636_66872_LeagueClientUxHelper-utility.log"
        ));
    }

    #[test]
    fn rejects_invalid_standard_format() {
        let error = parse_standard_lockfile(Path::new("lockfile"), "bad").unwrap_err();

        assert!(matches!(
            error,
            LcuConnectionError::InvalidLockfileFormat { .. }
        ));
    }
}
