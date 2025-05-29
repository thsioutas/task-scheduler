use api::configure_app;
use clap::Parser;
use scheduler::{InMemoryStorage, PostgresStorage, TaskScheduler, TaskStorageBackend};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::fmt;

mod api;
mod scheduler;

const DEFAULT_MAX_TASKS: usize = 100;
const DEFAULT_LISTENING_PORT: u16 = 3030;

const DEFAULT_POSTGRES_HOST: &str = "localhost";
const DEFAULT_POSTGRES_PORT: u16 = 5432;

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

    dotenv::dotenv().ok();

    // Setup logger
    let subscriber = fmt().with_max_level(args.verbosity).finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    let task_storage = if let Ok(db_name) = std::env::var("POSTGRES_DB") {
        // Panic if postgres init fails
        let db_user = std::env::var("POSTGRES_USER").expect("Missing POSTGRES_USER");
        let db_pass = std::env::var("POSTGRES_PASSWORD").expect("Missing POSTGRES_PASSWORD");
        let db_host =
            std::env::var("POSTGRES_HOST").unwrap_or_else(|_| DEFAULT_POSTGRES_HOST.to_string());
        let db_port =
            std::env::var("POSTGRES_PORT").unwrap_or_else(|_| DEFAULT_POSTGRES_PORT.to_string());
        let db_url = format!(
            "postgres://{}:{}@{}:{}/{}",
            db_user, db_pass, db_host, db_port, db_name
        );
        let backend = PostgresStorage::new(&db_url).await.unwrap();
        TaskStorageBackend::Postgres(backend)
    } else {
        let backend = InMemoryStorage::new();
        TaskStorageBackend::InMemory(backend)
    };
    let taks = std::env::var("MAX_TASKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_MAX_TASKS);
    let task_scheduler = TaskScheduler::new(taks, task_storage);
    info!("Task scheduler started");
    // Configure API
    let app = configure_app(Arc::new(task_scheduler));

    // Start HTTP server on localhost:<port>
    let port =
        std::env::var("LISTENING_PORT").unwrap_or_else(|_| DEFAULT_LISTENING_PORT.to_string());
    let binding_address = format!("localhost:{}", port);
    let listener = tokio::net::TcpListener::bind(binding_address)
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
