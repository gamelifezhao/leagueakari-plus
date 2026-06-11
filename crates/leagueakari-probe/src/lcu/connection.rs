use std::{
    env, fs,
    path::{Path, PathBuf},
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

    connections.extend(discover_standard_lockfiles()?);
    connections.extend(discover_from_logs()?);
    dedupe_connections(&mut connections);

    if connections.is_empty() {
        Err(LcuConnectionError::NotFound)
    } else {
        Ok(connections)
    }
}

fn discover_standard_lockfiles() -> Result<Vec<LcuConnection>, LcuConnectionError> {
    standard_lockfile_paths()
        .into_iter()
        .filter(|path| {
            path.is_file()
                && path
                    .metadata()
                    .map(|metadata| metadata.len() > 0)
                    .unwrap_or(false)
        })
        .map(read_standard_lockfile)
        .collect()
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
        source: "standard lockfile".to_string(),
        path: path.to_path_buf(),
        pid: Some(pid),
        port,
        password: parts[3].to_string(),
        protocol: parts[4].to_string(),
    })
}

fn discover_from_logs() -> Result<Vec<LcuConnection>, LcuConnectionError> {
    latest_log_paths()
        .into_iter()
        .take(MAX_LOG_FILES)
        .map(|path| {
            let content = read_log_lossy(&path)?;
            parse_log_connections(&path, &content)
        })
        .collect::<Result<Vec<_>, LcuConnectionError>>()
        .map(|groups| {
            groups
                .into_iter()
                .flatten()
                .take(MAX_LOG_CONNECTIONS)
                .collect()
        })
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
            source: "LeagueClientUx log arguments".to_string(),
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

                if file_name.contains("LeagueClientUx") && file_name.ends_with(".log") {
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

    roots
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
    fn rejects_invalid_standard_format() {
        let error = parse_standard_lockfile(Path::new("lockfile"), "bad").unwrap_err();

        assert!(matches!(
            error,
            LcuConnectionError::InvalidLockfileFormat { .. }
        ));
    }
}
