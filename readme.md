# Rust Async HTTP Task Scheduler

## Overview

This project is an asynchronous task scheduler built with Rust that handles various tasks,
including Solana blockchain transactions and Mandelbrot fractal generation.

## Setup & Usage
1. Clone the repository:
```
git clone https://github.com/your-repo/rust-task-scheduler.git
cd rust-task-scheduler
```

2. Build and start the server
```
cargo run -- --verbosity 5
```

### Example requests
#### Submit a Mandelbrot fractal generation task:
```
curl -X POST http://localhost:3030/scheduler/tasks \
     -H "Content-Type: application/json" \
     -d '{
          "type": "MandelbrotCompute",
          "width": 800,
          "height": 600,
          "max_iterations": 500
        }'
```

#### Submit a Mandelbrot fractal generation task:
```
curl -X POST http://localhost:3030/scheduler/tasks \
     -H "Content-Type: application/json" \
     -d '{
          "type": "SolanaTransfer",
          "payer": "YourSolanaAddress",
          "recipient": "YourSolanaAddress",
          "amount": 1000000
        }'
```

## Next Steps

1. Persist Completed Tasks – Store task history in a PostgreSQL database.
2. Add Task Progress Reporting – Use WebSockets or periodic polling for task monitoring.
3. Improve Error Handling
5. Task Prioritization & Scheduling – Implement priority-based execution for tasks.
6. Docker Support – Containerize the application for easier deployment.
7. Implement Solana Transfer task
8. Configuration File Support