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

    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let options = lcu::ProbeOptions {
        raw: args.iter().any(|arg| arg == "--raw"),
        watch: args.iter().any(|arg| arg == "--watch"),
    };

    lcu::run_probe(options).await
}
