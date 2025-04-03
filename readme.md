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

## Design
For some design choices see [here](design.md).

## Next Steps

- [ ] Persist Completed Tasks – Store task history in a PostgreSQL database.
- [ ] Add Task Progress Reporting – Use WebSockets or periodic polling for task monitoring.
- [ ] Improve Error Handling
- [ ] Task Prioritization & Scheduling – Implement priority-based execution for tasks.
- [ ] Docker Support – Containerize the application for easier deployment.
- [ ] Implement Solana Transfer task
- [ ] Configuration File Support
- [ ] OpenAPI docs