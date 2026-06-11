use std::{
    env, fs,
    path::{Path, PathBuf},
};

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Lockfile {
    pub path: PathBuf,
    pub pid: u32,
    pub port: u16,
    pub password: String,
    pub protocol: String,
}

#[derive(Debug, Error)]
pub enum LockfileError {
    #[error(
        "League Client lockfile was not found. Start the League client, then run the probe again."
    )]
    NotFound,
    #[error("failed to read lockfile at {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid lockfile format at {path}: expected LeagueClient:pid:port:password:protocol")]
    InvalidFormat { path: PathBuf },
    #[error("invalid pid in lockfile at {path}: {value}")]
    InvalidPid { path: PathBuf, value: String },
    #[error("invalid port in lockfile at {path}: {value}")]
    InvalidPort { path: PathBuf, value: String },
}

pub fn discover() -> Result<Lockfile, LockfileError> {
    candidate_paths()
        .into_iter()
        .find_map(|path| {
            if path.is_file() {
                Some(read_lockfile(path))
            } else {
                None
            }
        })
        .unwrap_or(Err(LockfileError::NotFound))
}

fn read_lockfile(path: PathBuf) -> Result<Lockfile, LockfileError> {
    let content = fs::read_to_string(&path).map_err(|source| LockfileError::Read {
        path: path.clone(),
        source,
    })?;
    parse_lockfile(&path, content.trim())
}

fn parse_lockfile(path: &Path, content: &str) -> Result<Lockfile, LockfileError> {
    let parts: Vec<&str> = content.split(':').collect();
    if parts.len() != 5 || parts[0] != "LeagueClient" {
        return Err(LockfileError::InvalidFormat {
            path: path.to_path_buf(),
        });
    }

    let pid = parts[1]
        .parse::<u32>()
        .map_err(|_| LockfileError::InvalidPid {
            path: path.to_path_buf(),
            value: parts[1].to_string(),
        })?;
    let port = parts[2]
        .parse::<u16>()
        .map_err(|_| LockfileError::InvalidPort {
            path: path.to_path_buf(),
            value: parts[2].to_string(),
        })?;

    Ok(Lockfile {
        path: path.to_path_buf(),
        pid,
        port,
        password: parts[3].to_string(),
        protocol: parts[4].to_string(),
    })
}

fn candidate_paths() -> Vec<PathBuf> {
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

    for path in [
        r"C:\Riot Games\League of Legends\lockfile",
        r"C:\Program Files\Riot Games\League of Legends\lockfile",
        r"C:\Program Files (x86)\Riot Games\League of Legends\lockfile",
        r"D:\Riot Games\League of Legends\lockfile",
        r"E:\Riot Games\League of Legends\lockfile",
    ] {
        paths.push(PathBuf::from(path));
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_lockfile() {
        let lockfile =
            parse_lockfile(Path::new("lockfile"), "LeagueClient:1234:5678:secret:https").unwrap();

        assert_eq!(lockfile.pid, 1234);
        assert_eq!(lockfile.port, 5678);
        assert_eq!(lockfile.password, "secret");
        assert_eq!(lockfile.protocol, "https");
    }

    #[test]
    fn rejects_invalid_format() {
        let error = parse_lockfile(Path::new("lockfile"), "bad").unwrap_err();

        assert!(matches!(error, LockfileError::InvalidFormat { .. }));
    }
}
