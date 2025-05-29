use crate::scheduler::TaskStatus;
use async_trait::async_trait;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::collections::HashMap;
use tokio::sync::RwLock as AsyncRwLock;
use uuid::Uuid;

#[async_trait]
pub trait TaskStorage: Send + Sync + 'static {
    async fn insert(&self, task_id: Uuid, status: TaskStatus);
    async fn update(&self, task_id: Uuid, status: TaskStatus);
    async fn get_tasks(&self) -> HashMap<Uuid, TaskStatus>;
}

pub enum TaskStorageBackend {
    InMemory(InMemoryStorage),
    Postgres(PostgresStorage),
}

#[async_trait]
impl TaskStorage for TaskStorageBackend {
    async fn insert(&self, task_id: Uuid, status: TaskStatus) {
        use TaskStorageBackend::*;
        match self {
            InMemory(mem) => mem.insert(task_id, status).await,
            Postgres(pg) => pg.insert(task_id, status).await,
        }
    }

    async fn update(&self, task_id: Uuid, status: TaskStatus) {
        use TaskStorageBackend::*;
        match self {
            InMemory(mem) => mem.update(task_id, status).await,
            Postgres(pg) => pg.update(task_id, status).await,
        }
    }

    async fn get_tasks(&self) -> HashMap<Uuid, TaskStatus> {
        use TaskStorageBackend::*;
        match self {
            InMemory(mem) => mem.get_tasks().await,
            Postgres(pg) => pg.get_tasks().await,
        }
    }
}

pub struct InMemoryStorage {
    tasks: AsyncRwLock<HashMap<Uuid, TaskStatus>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            tasks: AsyncRwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl TaskStorage for InMemoryStorage {
    async fn insert(&self, task_id: Uuid, status: TaskStatus) {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id, status);
    }

    async fn update(&self, task_id: Uuid, status: TaskStatus) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            *task = status;
        }
    }

    async fn get_tasks(&self) -> HashMap<Uuid, TaskStatus> {
        self.tasks.read().await.clone()
    }
}

pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }
}

#[async_trait]
impl TaskStorage for PostgresStorage {
    async fn insert(&self, task_id: Uuid, status: TaskStatus) {
        let (status_str, error_message) = match status.clone() {
            TaskStatus::Ongoing => ("Ongoing".to_string(), None),
            TaskStatus::Completed => ("Completed".to_string(), None),
            TaskStatus::Failed(msg) => ("Failed".to_string(), Some(msg)),
        };

        let _ = sqlx::query("INSERT INTO tasks (id, status, error_message) VALUES ($1, $2, $3) ON CONFLICT (id) DO UPDATE SET status = EXCLUDED.status, error_message = EXCLUDED.error_message")
            .bind(task_id)
            .bind(status_str)
            .bind(error_message)
            .execute(&self.pool)
            .await;
    }

    async fn update(&self, task_id: Uuid, status: TaskStatus) {
        self.insert(task_id, status).await; // Same logic — upsert
    }

    async fn get_tasks(&self) -> HashMap<Uuid, TaskStatus> {
        let rows = sqlx::query("SELECT id, status, error_message FROM tasks")
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        rows.into_iter()
            .map(|row| {
                let id: Uuid = row.get("id");
                let status_str: String = row.get("status");
                let error_message: Option<String> = row.get("status");
                let status = match status_str.as_str() {
                    "Ongoing" => TaskStatus::Ongoing,
                    "Completed" => TaskStatus::Completed,
                    "Failed" => {
                        TaskStatus::Failed(error_message.unwrap_or("Unknown status".to_string()))
                    }
                    _ => TaskStatus::Failed("Unknown status".to_string()),
                };
                (id, status)
            })
            .collect()
    }
}
