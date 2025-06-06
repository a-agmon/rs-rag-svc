# LangGraph-like service template in Rust

A **LangGraph-like framework** built with Rust that demonstrates workflow orchestration for AI agents. This example showcases how to build a service endpoint that can trigger and manage high-performance, composable AI workflows using:

- **[Axum](https://github.com/tokio-rs/axum)** - Fast, ergonomic web framework for speed and reliability (as service endpoint)
- **[Rig](https://github.com/0xPlaygrounds/rig)** - Rust library for LLM communication and agent building
- **[task-graph](https://github.com/a-agmon/rs-task-graph)** - Custom task execution engine for workflow orchestration

## What is this?

This repository demonstrates how to build a **LangGraph-inspired workflow system** in Rust, providing:

- **Composable AI Workflows**: Chain together AI tasks using a graph-based execution model
- **High Performance**: Leveraging Rust's performance and Axum's speed for production workloads  
- **Type Safety**: Full compile-time guarantees for workflow definitions and data flow
- **LLM Integration**: Seamless communication with language models via the Rig crate
- **Production Ready**: Built-in error handling, logging, and observability

## Architecture

The framework follows a **task graph execution model** similar to LangGraph:

1. **Tasks** - Individual units of work (e.g., query enhancement, answer generation)
2. **Workflow Graph** - Defines task dependencies and execution order
3. **Context** - Shared state that flows between tasks via which tasks can communicate
4. **Agent Integration** - LLM-powered tasks using Rig for model communication

### Example Workflow

Example of a query enhancement task
```rust
#[derive(Debug, Clone)]
pub struct EnhanceQueryTask {
    query: String,
}
impl EnhanceQueryTask {
    pub fn new(query: String) -> Self {
        Self { query }
    }
}

#[async_trait]
impl Task for EnhanceQueryTask {
    async fn run(&self, context: Context) -> Result<(), GraphError> {
        let enhanced_query = enhance_query(self.query.clone())
            .await
            .map_err(|e| GraphError::TaskExecutionFailed(e.to_string()))?;
        info!("Enhanced query: {}", enhanced_query);

        context
            .set(context_vars::ENHANCED_QUERY, enhanced_query)
            .await;
        info!("Enhanced query set in context");

        Ok(())
    }
}

const ENHANCE_QUERY_PROMPT: &str = r#"You are a search assistant.
Improve the user query for retrieval.
Rewrite it and add keywords so that a similarity search will find more relevant documents.
Keep it short (one sentence).
"#;

async fn enhance_query(query: String) -> anyhow::Result<String> {
    let agent = get_llm_agent(ENHANCE_QUERY_PROMPT)?;
    let response = agent.prompt(query).await?;
    Ok(response)
}
```
Chain and execute all tasks. Get output from the context at the end of the flow.
```rust
// Create a workflow that enhances a query then generates an answer
let mut graph = TaskGraph::new();
let enhance_task = EnhanceQueryTask::new(query);
let generate_task = GenerateAnswerTask;
graph.add_edge(enhance_task, generate_task)?;

// Execute the workflow
graph.execute().await?;
```

## Features

### Workflow Orchestration
- **Task Graph Execution**: Define and execute complex AI workflows with dependencies
- **Context Management**: Shared state that flows seamlessly between workflow tasks
- **Async Task Execution**: Non-blocking, concurrent task processing

### AI Agent Integration  
- **LLM Communication**: Built-in support for OpenRouter and other LLM providers via Rig
- **Agent Workflows**: Compose multi-step AI processes (query enhancement → answer generation)
- **Flexible Prompting**: Configurable system prompts and agent behaviors

### High-Performance Web Service
- **Fast HTTP API**: Built on Axum for maximum throughput and low latency
- **Health Monitoring**: GET `/health` - Service status and health checks
- **Agent Endpoint**: POST `/api/agent1` - Execute AI workflows via REST API
- **CORS Support**: Cross-origin resource sharing for web applications

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo
- OpenRouter API key (for LLM communication) / or any other provider supported by Rig

### Setup

1. **Set up your environment**:
```bash
# Required: Set your OpenRouter API key
export OPENROUTER_API_KEY="your-api-key-here"

# Required: Set your Serper API key for search
export SERPER_API_KEY="your-serper-api-key-here"

# Optional: Configure host and port
export HOST="0.0.0.0"
export PORT="8080"
```

2. **Install dependencies and run**:
```bash
cargo run
```

The service will start on `http://0.0.0.0:8080`

### Quick Test

Test the workflow execution:
```bash
curl -X POST http://localhost:8080/api/agent1 \
  -H "Content-Type: application/json" \
  -d '{"query": "Explain quantum computing in simple terms"}'
```

### Logging Configuration

The service uses structured logging with different levels. You can control the log level using the `RUST_LOG` environment variable:

```bash
# Default logging (debug level for our service)
cargo run

# Info level only
RUST_LOG=info cargo run

# Debug level with HTTP tracing
RUST_LOG=rs_rag_svc=debug,tower_http=debug cargo run

# Trace level for maximum verbosity
RUST_LOG=trace cargo run
```

### Environment Configuration

You can configure the service using environment variables:

```bash
# Change port and host
HOST=127.0.0.1 PORT=3000 cargo run

# Custom logging
RUST_LOG=info cargo run
```

## API Endpoints

### Health Check
```bash
curl http://localhost:8080/health
```

**Response:**
```json
{
  "status": "ok",
  "message": "Service is healthy"
}
```

### Agent Endpoint
```bash
curl -X POST http://localhost:8080/api/agent1 \
  -H "Content-Type: application/json" \
  -d '{"query": "What is the weather like?"}'
```

**Request Body:**
```json
{
  "query": "Your question here"
}
```

**Response:**
```json
{
  "answer": "Processed your query: What is the weather like?"
}
```

**Error Handling:**
- Returns `400 Bad Request` if query is empty or only whitespace
- Structured error responses with error type and message

## Development

To run in development mode with automatic reloading:
```bash
cargo watch -x run
```

For detailed HTTP request logging:
```bash
RUST_LOG=rs_rag_svc=debug,tower_http=debug cargo watch -x run
```

### Running Tests
```bash
cargo test
```

## Project Structure

```
src/
├── main.rs                    # Application entry point
├── lib.rs                     # Library root with module declarations
├── app.rs                     # Application setup and initialization
├── config.rs                  # Configuration management
├── error.rs                   # Custom error types and handling
├── handlers.rs                # HTTP request handlers
├── models.rs                  # Request/response data structures
├── routes.rs                  # Route definitions and organization
└── agent_workflow/            # LangGraph-like workflow implementation
    ├── mod.rs                 # Workflow orchestration and LLM agent setup
    ├── enhance.rs             # Query enhancement task
    └── generate.rs            # Answer generation task
```

### Module Responsibilities

#### Core Web Service
- **`main.rs`**: Binary entry point, starts the server
- **`app.rs`**: Application initialization, tracing setup, router creation
- **`config.rs`**: Environment-based configuration management
- **`error.rs`**: Custom error types with proper HTTP status mapping
- **`handlers.rs`**: HTTP handlers that orchestrate workflow execution
- **`models.rs`**: Data structures for API requests/responses with validation
- **`routes.rs`**: Route organization and endpoint definitions

#### Workflow Engine (`agent_workflow/`)
- **`mod.rs`**: Workflow graph creation, LLM agent initialization, and context management
- **`enhance.rs`**: Task for enhancing user queries using LLM reasoning
- **`generate.rs`**: Task for generating final answers based on enhanced queries

### Key Dependencies

- **`rig-core`**: LLM communication and agent building
- **`task-graph`**: Custom workflow orchestration engine
- **`axum`**: High-performance async web framework
- **`tokio`**: Async runtime for concurrent task execution

## Workflow Concepts

### LangGraph-like Design

This framework implements core LangGraph concepts in Rust:

- **Nodes (Tasks)**: Individual processing units that implement specific logic
- **Edges**: Dependencies between tasks that define execution order  
- **State (Context)**: Shared data that flows through the workflow
- **Conditional Routing**: Tasks can determine next steps based on results
- **Async Execution**: Non-blocking task processing with proper error handling

### Example: Query Enhancement Workflow

```rust
// 1. Create workflow graph
let mut graph = TaskGraph::new();

// 2. Define tasks
let enhance_task = EnhanceQueryTask::new(user_query);
let generate_task = GenerateAnswerTask;

// 3. Connect tasks (enhance → generate)
graph.add_edge(enhance_task, generate_task)?;

// 4. Execute workflow
graph.execute().await?;

// 5. Retrieve results from context
let answer = graph.context().get("answer").await?;
```

### Benefits over Python LangGraph

- **Performance**: Rust's zero-cost abstractions and memory safety
- **Type Safety**: Compile-time guarantees for workflow correctness
- **Concurrency**: Native async/await with excellent performance
- **Memory Efficiency**: No garbage collection overhead
- **Production Ready**: Built-in error handling and observability

