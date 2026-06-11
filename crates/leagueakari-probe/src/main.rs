mod lcu;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "leagueakari_probe=info".into()),
        )
        .init();

    let raw = std::env::args().any(|arg| arg == "--raw");
    lcu::run_probe(raw).await
}
