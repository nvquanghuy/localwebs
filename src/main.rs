mod config;
mod detector;
mod incremental_scanner;
mod models;
mod scanner;
mod web;

use clap::Parser;
use config::Config;
use incremental_scanner::IncrementalScanner;
use std::path::PathBuf;
use std::sync::Arc;
use web::{create_router, routes::AppState};

#[derive(Parser)]
#[command(name = "localwebs")]
#[command(version)]
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

    // Create incremental scanner
    let scanner = IncrementalScanner::new(config.clone());

    // Perform initial scan
    println!("🔍 Performing initial scan...");
    let initial_services = scanner.scan().await;
    println!("✅ Found {} services", initial_services.len());

    let state = Arc::new(AppState {
        config: config.clone(),
        scanner,
    });

    // Start background scanner (scans every 5 seconds for dev environment)
    let bg_state = Arc::clone(&state);
    tokio::spawn(async move {
        web::routes::start_background_scanner(bg_state, 5).await;
    });

    let app = create_router((*state).clone());

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("\n🌐 LocalWebs portal running at http://0.0.0.0:{}", port);
    println!("   Accessible from outside at http://<your-ip>:{}", port);
    println!("   📡 Incremental scanning active (5s interval)");
    println!("   Press Ctrl+C to stop\n");

    axum::serve(listener, app).await?;

    Ok(())
}
