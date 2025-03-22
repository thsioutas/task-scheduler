use axum::{routing::get, Router};
use tracing::trace;

async fn root() {
    trace!("Root");
}
async fn task_collection() {
    trace!("Get task collection");
}
async fn task_create() {
    trace!("Add a new task");
}

pub fn configure_app() -> Router {
    Router::new()
        .route("/scheduler", get(root))
        .route("/scheduler/tasks", get(task_collection).post(task_create))
}
