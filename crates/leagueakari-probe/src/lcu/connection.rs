use std::{
    env, fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

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

pub fn discover() -> Result<LcuConnection, LcuConnectionError> {
    if let Some(result) = discover_standard_lockfile() {
        return result;
    }

    if let Some(result) = discover_from_logs() {
        return result;
    }

    Err(LcuConnectionError::NotFound)
}

fn discover_standard_lockfile() -> Option<Result<LcuConnection, LcuConnectionError>> {
    standard_lockfile_paths()
        .into_iter()
        .find(|path| {
            path.is_file()
                && path
                    .metadata()
                    .map(|metadata| metadata.len() > 0)
                    .unwrap_or(false)
        })
        .map(read_standard_lockfile)
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

fn discover_from_logs() -> Option<Result<LcuConnection, LcuConnectionError>> {
    latest_log_paths()
        .into_iter()
        .filter_map(|path| match read_log_lossy(&path) {
            Ok(content) => parse_log_connection(&path, &content).transpose(),
            Err(error) => Some(Err(error)),
        })
        .next()
}

fn parse_log_connection(
    path: &Path,
    content: &str,
) -> Result<Option<LcuConnection>, LcuConnectionError> {
    let port = find_flag_value(content, "--app-port=").map(|value| parse_port(path, value));
    let token = find_flag_value(content, "--remoting-auth-token=");

    match (port, token) {
        (Some(port), Some(token)) => Ok(Some(LcuConnection {
            source: "LeagueClientUx log arguments".to_string(),
            path: path.to_path_buf(),
            pid: None,
            port: port?,
            password: token.to_string(),
            protocol: "https".to_string(),
        })),
        _ => Ok(None),
    }
}

fn find_flag_value<'a>(content: &'a str, flag: &str) -> Option<&'a str> {
    content
        .match_indices(flag)
        .last()
        .and_then(|(index, _)| content[index + flag.len()..].split_whitespace().next())
        .map(|value| value.trim_matches('"'))
        .filter(|value| !value.is_empty())
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
        let connection = parse_log_connection(Path::new("LeagueClientUx.log"), content)
            .unwrap()
            .unwrap();

        assert_eq!(connection.port, 58425);
        assert_eq!(connection.password, "token-value");
        assert_eq!(connection.protocol, "https");
    }

    #[test]
    fn uses_last_log_arguments() {
        let content = "--remoting-auth-token=old --app-port=11111\n--remoting-auth-token=new --app-port=22222";
        let connection = parse_log_connection(Path::new("LeagueClientUx.log"), content)
            .unwrap()
            .unwrap();

        assert_eq!(connection.port, 22222);
        assert_eq!(connection.password, "new");
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
