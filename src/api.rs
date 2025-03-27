use crate::scheduler::*;
use axum::{routing::get, Json, Router};
use serde::Serialize;
use tracing::trace;
use uuid::Uuid;

async fn root() -> Json<RootResponse> {
    trace!("Root");
    Json(RootResponse {
        id: "SchedulerRootEndpoint".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        tasks: "/scheduler/tasks".to_string(),
    })
}

async fn task_collection() -> Json<TasksCollectionResponse> {
    trace!("Get task collection");
    Json(TasksCollectionResponse {
        id: "TaskCollection".to_string(),
    })
}

async fn task_create(
    Json(payload): Json<Task>,
    task_scheduler: TaskScheduler,
) -> Json<TaskResponse> {
    let task = payload.into();
    trace!("Add a new task: {:?}", task);
    task_scheduler.submit_task(task).await;
    let id = Uuid::new_v4().to_string();
    Json(TaskResponse {
        id,
        message: Some("Task received and is processing".to_string()),
    })
}

pub fn configure_app(task_scheduler: TaskScheduler) -> Router {
    Router::new().route("/scheduler", get(root)).route(
        "/scheduler/tasks",
        get(task_collection).post(move |body| task_create(body, task_scheduler)),
    )
}

#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
struct TaskResponse {
    id: String,
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct TasksCollectionResponse {
    id: String,
}

#[derive(Debug, Serialize)]
struct RootResponse {
    id: String,
    version: String,
    tasks: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_task_create() {
        let task_request = Task::SolanaTransfer(SolanaTransferTask {
            payer: "PayerPubKeyHere".to_string(),
            recipient: "RecipientPubKeyHere".to_string(),
            amount: 2.1,
        });
        let expected_response = TaskResponse {
            id: "1".to_string(),
            message: Some("Task received and is processing".to_string()),
        };
        let task_scheduler = TaskScheduler::new(5);
        let response = task_create(Json(task_request), task_scheduler).await.0;
        assert_eq!(expected_response, response);
    }
}
