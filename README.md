# RS RAG Service

A simple Axum web service with health check and agent endpoints, featuring comprehensive logging with tracing.

## Features

- **Health Check**: GET `/health` - Returns service status
- **Agent Endpoint**: POST `/api/agent1` - Processes queries and returns answers
- **Comprehensive Logging**: Structured logging with tracing and tracing-subscriber
- **HTTP Request Tracing**: Automatic logging of all HTTP requests and responses
- **CORS Support**: Cross-origin resource sharing enabled
- **Production-Ready Architecture**: Modular design with proper error handling

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo

### Running the Service

1. Install dependencies and run:
```bash
cargo run
```

The service will start on `http://0.0.0.0:8080`

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
├── main.rs           # Application entry point
├── lib.rs            # Library root with module declarations
├── app.rs            # Application setup and initialization
├── config.rs         # Configuration management
├── error.rs          # Custom error types and handling
├── handlers.rs       # HTTP request handlers
├── models.rs         # Request/response data structures
└── routes.rs         # Route definitions and organization
```

### Module Responsibilities

- **`main.rs`**: Binary entry point, starts the server
- **`app.rs`**: Application initialization, tracing setup, router creation
- **`config.rs`**: Environment-based configuration management
- **`error.rs`**: Custom error types with proper HTTP status mapping
- **`handlers.rs`**: Business logic for each endpoint with comprehensive logging
- **`models.rs`**: Data structures for API requests/responses with validation
- **`routes.rs`**: Route organization and endpoint definitions

## Logging Features

- **Request/Response Logging**: All HTTP requests are automatically logged
- **Structured Logs**: JSON-structured logs for easy parsing
- **Error Tracking**: Comprehensive error logging with context
- **Performance Monitoring**: Request duration and status code tracking
- **Configurable Levels**: Debug, info, warn, and error logging 