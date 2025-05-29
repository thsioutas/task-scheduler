use crate::scheduler::*;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use tracing::trace;

fn scheduler_url(path: &str) -> String {
    format!("/scheduler{}", path)
}

async fn root() -> Json<RootResponse> {
    trace!("Root");
    Json(RootResponse {
        id: "SchedulerRootEndpoint".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        tasks: scheduler_url("/tasks"),
    })
}

async fn task_collection(
    State(task_scheduler): State<Arc<TaskScheduler>>,
) -> Json<TasksCollectionResponse> {
    trace!("Get task collection");
    let ids = task_scheduler.get_tasks().await;
    let tasks = ids
        .iter()
        .map(|id| scheduler_url(&format!("/tasks/{}", id)))
        .collect();
    Json(TasksCollectionResponse { tasks })
}

async fn task_create(
    State(task_scheduler): State<Arc<TaskScheduler>>,
    Json(payload): Json<TaskPayload>,
) -> Result<Json<TaskCreatedResponse>, TaskSchedulerError> {
    let task = payload;
    trace!("Add a new task: {:?}", task);
    let id = task_scheduler.submit_task(task).await?;
    let url = scheduler_url(&format!("/tasks/{}", id));

    Ok(Json(TaskCreatedResponse { url }))
}

async fn get_task(
    State(task_scheduler): State<Arc<TaskScheduler>>,
    Path(id): Path<String>,
) -> Result<Json<TaskResponse>, TaskSchedulerError> {
    trace!("Get task info: {}", id);
    let status = task_scheduler.get_task_status(&id).await?;

    Ok(Json(TaskResponse { id, status }))
}

pub fn configure_app(task_scheduler: Arc<TaskScheduler>) -> Router {
    Router::new()
        .route("/scheduler", get(root))
        .route("/scheduler/tasks", get(task_collection).post(task_create))
        .route("/scheduler/tasks/{id}", get(get_task))
        .with_state(task_scheduler) // Injects `task_scheduler` into all handlers
}

#[derive(Debug, Serialize)]
struct TaskCreatedResponse {
    url: String,
}

#[derive(Debug, Serialize)]
struct TaskResponse {
    id: String,
    status: TaskStatus,
}

#[derive(Debug, Serialize)]
struct TasksCollectionResponse {
    tasks: Vec<String>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
struct RootResponse {
    id: String,
    version: String,
    tasks: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_url() {
        assert_eq!("/scheduler", scheduler_url(""));
        assert_eq!("/scheduler/tasks", scheduler_url("/tasks"));
    }

    #[tokio::test]
    async fn test_api_root() {
        let response = root().await.0;
        let expected_response = RootResponse {
            id: "SchedulerRootEndpoint".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            tasks: scheduler_url("/tasks"),
        };
        assert_eq!(response, expected_response);
    }

    #[tokio::test]
    async fn test_api_task_collection() {
        let task_scheduler = Arc::new(TaskScheduler::new_mock());

        let response = task_collection(axum::extract::State(task_scheduler.clone()))
            .await
            .0;

        assert!(response.tasks.is_empty());
    }

    #[tokio::test]
    async fn test_api_task_create() {
        let task_scheduler = Arc::new(TaskScheduler::new_mock());
        let task_type = TaskType::SolanaTransfer(SolanaTransferTask {
            net: "devnet".to_string(),
            recipient: "recipient_pubkey".to_string(),
            amount: 2.1,
        });
        let task_request = TaskPayload {
            priority: 1,
            task_type,
        };

        let response = task_create(
            axum::extract::State(task_scheduler.clone()),
            Json(task_request),
        )
        .await
        .unwrap()
        .0;

        assert!(response.url.starts_with(&scheduler_url("/tasks/")));
    }
}
