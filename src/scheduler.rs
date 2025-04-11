use async_trait::async_trait;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use image::{ImageBuffer, Rgb};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::sync::RwLock as AsyncRwLock;
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

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Task {
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

#[derive(Clone)]
pub struct TaskScheduler<T> {
    tx: mpsc::Sender<(String, Task)>,
    task_storage: Arc<T>,
}

impl<T: TaskStorage> TaskScheduler<T> {
    /// Initialize the scheduler with a channel
    pub fn new(max_tasks: usize, task_storage: T) -> Self {
        let (tx, rx) = mpsc::channel(max_tasks);
        let task_storage = Arc::new(task_storage);
        tokio::spawn(run_scheduler(rx, Arc::clone(&task_storage)));
        Self { tx, task_storage }
    }

    /// Submit a new task
    pub async fn submit_task(&self, task: Task) -> Result<String, TaskSchedulerError> {
        let task_id = Uuid::new_v4().to_string();
        self.tx.send((task_id.clone(), task)).await?;
        // Insert task as 'Ongoing'
        self.task_storage
            .insert(task_id.clone(), TaskStatus::Ongoing)
            .await;
        info!("Task {} submitted to queue.", task_id);
        Ok(task_id)
    }

    pub async fn get_task_status(&self, task_id: &str) -> TaskStatus {
        let tasks = self.task_storage.get_tasks().await;
        tasks.get(task_id).unwrap().clone()
    }

    pub async fn get_tasks(&self) -> Vec<String> {
        let tasks = self.task_storage.get_tasks().await;
        tasks.into_iter().map(|task| task.0).collect()
    }
}

async fn run_scheduler<T: TaskStorage>(
    mut rx: mpsc::Receiver<(String, Task)>,
    task_storage: Arc<T>,
) {
    while let Some((task_id, task)) = rx.recv().await {
        let storage = Arc::clone(&task_storage);
        tokio::spawn(async move {
            let result = match task {
                Task::SolanaTransfer(solana_transfer_task) => {
                    info!("Processing SOL transfer for {}", task_id);
                    process_solana_transfer(solana_transfer_task).await
                }
                Task::MandelbrotCompute(mandelbrot_task) => {
                    info!("Computing Mandelbrot set for {}", task_id);
                    process_mandelbrot_task(task_id.clone(), mandelbrot_task).await
                }
            };

            // Update task status based on result
            let new_status = match result {
                Ok(_) => TaskStatus::Completed,
                Err(err) => TaskStatus::Failed(err),
            };
            storage.update(task_id, new_status).await;
        });
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
    save_image(img, &filename);

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
fn save_image(img: ImageBuffer<Rgb<u8>, Vec<u8>>, filename: &str) {
    img.save(filename).unwrap();
}

#[derive(Debug, Clone, Serialize)]
pub enum TaskStatus {
    Ongoing,
    Completed,
    Failed(String),
}

#[async_trait]
pub trait TaskStorage: Send + Sync + 'static {
    async fn insert(&self, task_id: String, status: TaskStatus);
    async fn update(&self, task_id: String, status: TaskStatus);
    async fn get_tasks(&self) -> HashMap<String, TaskStatus>;
}

pub struct InMemoryStorage {
    // TODO: String or Uuid?
    tasks: AsyncRwLock<HashMap<String, TaskStatus>>,
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
    async fn insert(&self, task_id: String, status: TaskStatus) {
        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id, status);
    }

    async fn update(&self, task_id: String, status: TaskStatus) {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(&task_id) {
            *task = status;
        }
    }

    async fn get_tasks(&self) -> HashMap<String, TaskStatus> {
        self.tasks.read().await.clone()
    }
}

#[derive(Debug, Error)]
pub enum TaskSchedulerError {
    #[error("Failed to send task to queue")]
    TaskQueueError(#[from] tokio::sync::mpsc::error::SendError<(String, Task)>),
}

impl IntoResponse for TaskSchedulerError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::TaskQueueError(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Task queue error: {:?}", err),
            ),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    impl TaskScheduler<InMemoryStorage> {
        pub fn new_mock() -> Self {
            let task_storage = Arc::new(InMemoryStorage::new());
            let (tx, mut rx) = mpsc::channel(10);

            tokio::spawn(async move { while let Some((_task_id, _task)) = rx.recv().await {} });
            Self { tx, task_storage }
        }
    }
}
