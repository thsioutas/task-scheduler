use image::{ImageBuffer, Rgb};
use rayon::prelude::*;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::info;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Task {
    SolanaTransfer(SolanaTransferTask),
    MandelbrotCompute(MandelbrotComputeTask),
}

#[derive(Debug, Deserialize)]
pub struct SolanaTransferTask {
    pub payer: String,
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
pub struct TaskScheduler {
    tx: mpsc::Sender<Task>,
}

impl TaskScheduler {
    /// Initialize the scheduler with a channel
    pub fn new(max_tasks: usize) -> Self {
        let (tx, rx) = mpsc::channel(max_tasks);
        tokio::spawn(run_scheduler(rx));
        Self { tx }
    }

    /// Submit a new task
    pub async fn submit_task(&self, task: Task) {
        self.tx.send(task).await.unwrap();
        info!("Task submitted to queue.");
    }
}

async fn run_scheduler(mut rx: mpsc::Receiver<Task>) {
    while let Some(task) = rx.recv().await {
        match task {
            Task::SolanaTransfer(solana_transfer_task) => {
                info!("A SOL transfer has been requested");
            }
            Task::MandelbrotCompute(mandelbrot_task) => {
                println!("Generating Mandelbrot fractal...");
                std::thread::spawn(move || {
                    handle_mandelbrot_task(
                        mandelbrot_task.width,
                        mandelbrot_task.height,
                        mandelbrot_task.max_iterations,
                    );
                });
            }
        }
    }
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

/// Handle Mandelbrot Task in Scheduler
fn handle_mandelbrot_task(width: u32, height: u32, max_iterations: u32) {
    let img = generate_mandelbrot(width, height, max_iterations);
    // TODO: Use different name for image. Probably suffix with task's uuid
    save_image(img, "mandelbrot.png");
    println!("Mandelbrot fractal saved as 'mandelbrot.png'");
}
