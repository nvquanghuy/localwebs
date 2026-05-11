mod config;
mod detector;
mod models;
mod scanner;
mod web;

use clap::Parser;
use config::Config;
use std::path::PathBuf;
use web::{create_router, routes::AppState};

#[derive(Parser)]
#[command(name = "localwebs")]
#[command(about = "Service discovery portal for localhost", long_about = None)]
struct Cli {
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    #[arg(short, long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config = Config::load(&cli.config)?;
    let port = cli.port.unwrap_or(config.portal.port);

    let state = AppState { config };
    let app = create_router(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("🌐 LocalWebs portal running at http://0.0.0.0:{}", port);
    println!("   Accessible from outside at http://<your-ip>:{}", port);
    println!("   Press Ctrl+C to stop");

    axum::serve(listener, app).await?;

    Ok(())
}
