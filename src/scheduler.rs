use async_trait::async_trait;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use image::{ImageBuffer, Rgb};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    str::FromStr,
    time::Duration,
};
use thiserror::Error;
use tokio::sync::{Mutex as AsyncMutex, RwLock as AsyncRwLock};
use tracing::info;
use uuid::Uuid;

use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    message::Message,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction;

pub struct Task {
    id: Uuid,
    payload: TaskPayload,
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.payload.priority == other.payload.priority && self.id == other.id
    }
}

impl Eq for Task {}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match other.payload.priority.cmp(&self.payload.priority) {
            Ordering::Equal => self.id.cmp(&other.id),
            ord => ord,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TaskPayload {
    pub priority: u16,
    #[serde(flatten)]
    pub task_type: TaskType,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum TaskType {
    SolanaTransfer(SolanaTransferTask),
    MandelbrotCompute(MandelbrotComputeTask),
}

#[derive(Debug, Deserialize)]
pub struct SolanaTransferTask {
    pub net: String,
    pub recipient: String,
    pub amount: f64,
}

#[derive(Debug, Deserialize)]
pub struct MandelbrotComputeTask {
    pub width: u32,
    pub height: u32,
    pub max_iterations: u32,
}

pub struct TaskScheduler<T> {
    task_queue: Arc<AsyncMutex<BinaryHeap<Task>>>,
    task_storage: Arc<T>,
    max_tasks: usize,
}

impl<T: TaskStorage> TaskScheduler<T> {
    /// Initialize the scheduler with a channel
    pub fn new(max_tasks: usize, task_storage: T) -> Self {
        let task_storage = Arc::new(task_storage);
        let task_queue = Arc::new(AsyncMutex::new(BinaryHeap::new()));
        tokio::spawn(run_scheduler(
            Arc::clone(&task_queue),
            Arc::clone(&task_storage),
        ));

        Self {
            task_queue,
            task_storage,
            max_tasks,
        }
    }

    /// Submit a new task
    pub async fn submit_task(&self, payload: TaskPayload) -> Result<String, TaskSchedulerError> {
        let mut task_queue = self.task_queue.lock().await;
        if task_queue.len() >= self.max_tasks {
            return Err(TaskSchedulerError::QueueFull);
        }
        let task_id = Uuid::new_v4();
        let task = Task {
            id: task_id,
            payload,
        };
        task_queue.push(task);
        // Insert task as 'Ongoing'
        self.task_storage.insert(task_id, TaskStatus::Ongoing).await;
        info!("Task {} submitted to queue.", task_id);
        Ok(task_id.to_string())
    }

    /// Retrieve the task status of the provided task ID
    // TODO: Consider providing more info on status: priority, type, status
    pub async fn get_task_status(&self, task_id: &str) -> Result<TaskStatus, TaskSchedulerError> {
        let tasks = self.task_storage.get_tasks().await;
        let uuid = Uuid::from_str(task_id).map_err(|_| TaskSchedulerError::NoSuchId)?;
        let status = tasks.get(&uuid).ok_or(TaskSchedulerError::NoSuchId)?;
        Ok(status.clone())
    }

    /// Retrieve all available task ID
    pub async fn get_tasks(&self) -> Vec<Uuid> {
        let tasks = self.task_storage.get_tasks().await;
        tasks.into_iter().map(|task| task.0).collect()
    }
}

async fn run_scheduler<T: TaskStorage>(
    task_queue: Arc<AsyncMutex<BinaryHeap<Task>>>,
    task_storage: Arc<T>,
) {
    const POLL_INTERVAL: Duration = Duration::from_millis(100);
    loop {
        let maybe_task = {
            let mut queue = task_queue.lock().await;
            queue.pop()
        };

        if let Some(task) = maybe_task {
            let storage = Arc::clone(&task_storage);
            tokio::spawn(async move {
                let result = match task.payload.task_type {
                    TaskType::SolanaTransfer(solana_transfer_task) => {
                        info!("Processing SOL transfer for {}", task.id);
                        process_solana_transfer(solana_transfer_task).await
                    }
                    TaskType::MandelbrotCompute(mandelbrot_task) => {
                        info!("Computing Mandelbrot set for {}", task.id);
                        process_mandelbrot_task(task.id.to_string(), mandelbrot_task).await
                    }
                };

                // Update task status based on result
                let new_status = match result {
                    Ok(_) => TaskStatus::Completed,
                    Err(err) => TaskStatus::Failed(err),
                };
                storage.update(task.id, new_status).await;
            });
        } else {
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    }
}

async fn process_solana_transfer(task: SolanaTransferTask) -> Result<(), String> {
    // Validate amount
    if task.amount <= 0.0 {
        return Err("Invalid transfer amount".to_string());
    }

    let payer_keypair_path = std::env::var("SERVER_KEYPAIR_PATH")
        .map_err(|_| "SERVER_KEYPAIR_PATH is not set in env".to_string())?;

    // Load server keypair from file or env var
    let payer = read_keypair_file(&payer_keypair_path)
        .map_err(|e| format!("Failed to read server keypair: {e}"))?;

    // Create RPC client
    let client = RpcClient::new_with_commitment(task.net, CommitmentConfig::confirmed());

    // Parse recipient
    let recipient_pubkey = task
        .recipient
        .parse::<Pubkey>()
        .map_err(|_| "Invalid recipient pubkey".to_string())?;

    // Convert SOL to lamports
    let lamports = (task.amount * 1_000_000_000f64) as u64;

    // Get recent blockhash
    let blockhash = client
        .get_latest_blockhash()
        .map_err(|e| format!("Failed to get blockhash: {e}"))?;

    // Build transaction
    let instruction = instruction::transfer(&payer.pubkey(), &recipient_pubkey, lamports);
    let message = Message::new(&[instruction], Some(&payer.pubkey()));
    let transaction = Transaction::new(&[&payer], message, blockhash);

    // Send and confirm
    client
        .send_and_confirm_transaction(&transaction)
        .map_err(|e| format!("Transfer failed: {e}"))?;

    info!(
        "✅ Transferred {} SOL from {} to {}",
        task.amount,
        payer.pubkey(),
        task.recipient
    );

    Ok(())
}

async fn process_mandelbrot_task(
    task_id: String,
    task: MandelbrotComputeTask,
) -> Result<(), String> {
    let img = generate_mandelbrot(task.width, task.height, task.max_iterations);

    let filename = format!("mandelbrot_{}.png", task_id);
    save_image(img, &filename)?;

    info!("Mandelbrot fractal saved as {}", filename);
    Ok(())
}

/// Generate Mandelbrot fractal
fn generate_mandelbrot(
    width: u32,
    height: u32,
    max_iterations: u32,
) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let mut img = ImageBuffer::new(width, height);

    img.enumerate_pixels_mut()
        .par_bridge()
        .for_each(|(x, y, pixel)| {
            let cx = (x as f64 / width as f64) * 3.5 - 2.5;
            let cy = (y as f64 / height as f64) * 2.0 - 1.0;
            let mut zx = 0.0;
            let mut zy = 0.0;
            let mut iteration = 0;

            while zx * zx + zy * zy < 4.0 && iteration < max_iterations {
                let tmp = zx * zx - zy * zy + cx;
                zy = 2.0 * zx * zy + cy;
                zx = tmp;
                iteration += 1;
            }

            let color = (iteration * 255 / max_iterations) as u8;
            *pixel = Rgb([color, color, color]);
        });

    img
}

/// Save image to disk
fn save_image(img: ImageBuffer<Rgb<u8>, Vec<u8>>, filename: &str) -> Result<(), String> {
    img.save(filename).map_err(|err| err.to_string())?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub enum TaskStatus {
    Ongoing,
    // TODO: Consider providing some extra (task-specific) info on completed
    Completed,
    Failed(String),
}

#[async_trait]
pub trait TaskStorage: Send + Sync + 'static {
    async fn insert(&self, task_id: Uuid, status: TaskStatus);
    async fn update(&self, task_id: Uuid, status: TaskStatus);
    async fn get_tasks(&self) -> HashMap<Uuid, TaskStatus>;
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

#[derive(Debug, Error)]
pub enum TaskSchedulerError {
    #[error("Task queue is full")]
    QueueFull,
    #[error("The specific ID was not found")]
    NoSuchId,
}

impl IntoResponse for TaskSchedulerError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::QueueFull => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            Self::NoSuchId => (StatusCode::NOT_FOUND, self.to_string()),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    impl TaskScheduler<InMemoryStorage> {
        pub fn new_mock() -> Self {
            let task_storage = InMemoryStorage::new();
            TaskScheduler::new(10, task_storage)
        }
    }
}
