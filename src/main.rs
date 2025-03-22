use api::configure_app;
use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::fmt;

mod api;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The log verbosity level
    #[clap(short, long)]
    pub verbosity: Level,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Setup logger
    let subscriber = fmt().with_max_level(args.verbosity).finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    info!("Task scheduler started");

    // Configure API
    let app = configure_app();

    // Start HTTP server on localhost:3030
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3030")
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
