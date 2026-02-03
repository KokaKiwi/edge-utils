# fastly-dev-server

A local development server that emulates the Fastly Compute environment, providing a complete runtime for testing WebAssembly applications with persistent store support.

## Overview

`fastly-dev-server` wraps [Viceroy](https://github.com/fastly/Viceroy) and extends it with persistent storage capabilities, allowing you to develop and test Fastly Compute applications locally without deploying to Fastly's infrastructure. It provides REST APIs for managing Config Stores, KV Stores, and Secret Stores, with all data persisted to a local embedded database.

## Features

- **Local Compute Runtime**: Execute Fastly Compute WebAssembly modules locally
- **Persistent Storage**: All stores (Config, KV, Secret) are persisted using [redb](https://github.com/cberner/redb)
- **REST API**: Manage stores through a comprehensive REST API
- **OpenTelemetry Integration**: Built-in distributed tracing with OTLP export
- **Docker Compose Stack**: Complete observability setup with ClickHouse and Jaeger
- **Per-Request Isolation**: Each request gets a fresh Viceroy instance with stores loaded from the database

## Quick Start

### Prerequisites

- Rust toolchain with `wasm32-wasip1` target
- Fastly CLI (optional, for building with `fastly compute build`)

### Building

```bash
# From the workspace root
cargo build -p fastly-dev-server --release
```

### Running

```bash
# First, build your Fastly Compute application to WebAssembly
cd fastly/demo-server
cargo build --target wasm32-wasip1 --release

# Then run the dev server
cd ../..
cargo run -p fastly-dev-server -- target/wasm32-wasip1/release/fastly-demo-server.wasm
# Or using the built binary (at target/release/fastly-dev-server)
fastly-dev-server target/wasm32-wasip1/release/fastly-demo-server.wasm
```

Your Compute application will be available at `http://127.0.0.1:7676`
The management API will be available at `http://127.0.0.1:7677`

## Usage

### Command Line Options

```bash
fastly-dev-server [OPTIONS] <WASM_PATH>

Arguments:
  <WASM_PATH>  Path to the WebAssembly module

Options:
      --store-path <PATH>    Database file path [default: ./fastly-dev-store.db]
      --http-addr <ADDR>     Compute HTTP server address [default: 127.0.0.1:7676]
      --api-addr <ADDR>      API server address [default: 127.0.0.1:7677]
  -h, --help                 Print help
```

### Managing Stores via Fastly CLI

The dev-server implements Fastly's store management API, allowing you to use the official Fastly CLI to manage stores locally.

#### Configuring the Fastly CLI

To point the Fastly CLI at your local dev-server, modify `~/.config/fastly/config.toml` and set the `api_endpoint` key inside the `[fastly]` section:

```toml
[fastly]
api_endpoint = "http://127.0.0.1:7677"
```

This redirects all Fastly CLI store operations to your local dev-server instead of production Fastly infrastructure.

#### Config Stores

Config Stores hold string key-value pairs.

```bash
# Create a config store
fastly config-store create --name=my-config

# Set an item
fastly config-store-entry create --store-id=my-config --key=api-key --value=secret-key-123

# Get an item
fastly config-store-entry describe --store-id=my-config --key=api-key

# List all items
fastly config-store-entry list --store-id=my-config

# Delete a store
fastly config-store delete --store-id=my-config
```

#### KV Stores

KV Stores hold binary key-value pairs.

```bash
# Create a KV store
fastly kv-store create --name=my-cache

# Insert a key
fastly kv-store-entry create --store-id=my-cache --key=user:123 --value='{"name": "John Doe"}'

# Get a key
fastly kv-store-entry describe --store-id=my-cache --key=user:123

# List all keys
fastly kv-store-entry list --store-id=my-cache

# Delete a store
fastly kv-store delete --store-id=my-cache
```

#### Secret Stores

Secret Stores hold encrypted secret values.

```bash
# Create a secret store
fastly secret-store create --name=my-secrets

# Store a secret
fastly secret-store-entry create --store-id=my-secrets --name=db-password --secret=super-secret-password

# List secrets (returns metadata only, not values)
fastly secret-store-entry list --store-id=my-secrets

# Delete a store
fastly secret-store delete --store-id=my-secrets
```

## Architecture

### Dual Server Design

The dev-server runs two concurrent servers:

1. **Compute Server** (port 7676)
   - Executes the WebAssembly module using Viceroy
   - Each request creates a fresh Viceroy instance
   - Stores are loaded from the database and injected before execution
   - Handles standard HTTP requests to your Compute application

2. **API Server** (port 7677)
   - REST API for managing stores
   - Modifies the persistent database
   - Used during development to set up test data

### Storage Layer

All stores are persisted to a single [redb](https://github.com/cberner/redb) database file:

- **Metadata Table** (`__meta__`): Tracks all store definitions and their types
- **Store Tables**: Each store gets its own table for data
- **Atomic Operations**: All database operations are transactional

Store initialization happens per-request in `compute/stores.rs`, ensuring each Compute invocation sees the current database state.

### Request Lifecycle

```
HTTP Request → Compute Server (7676)
    ↓
Per-Request Middleware (connection info)
    ↓
Create Viceroy Instance from Template
    ↓
Load Stores from Database
    ↓
Execute WebAssembly Module
    ↓
Return Response
```

## Observability

The dev-server includes comprehensive OpenTelemetry instrumentation:

- Structured logging via `tracing`
- Distributed tracing with OTLP export (default: http://localhost:4318)
- Custom HTTP request/response tracing layer
- Graceful shutdown with trace provider cleanup

## Development

### Project Structure

```
fastly/dev-server/src/
├── api/              # REST API for store management
│   ├── stores/
│   │   ├── config/   # Config Store endpoints
│   │   ├── kv/       # KV Store endpoints
│   │   └── secret/   # Secret Store endpoints
│   └── util.rs       # API utilities
├── compute/          # Viceroy integration
│   ├── compat.rs     # HTTP version compatibility layer
│   ├── stores.rs     # Store initialization
│   └── util.rs       # Compute utilities
├── tables.rs         # Database schema definitions
├── trace.rs          # OpenTelemetry setup
└── main.rs           # Entry point
```

## Dependencies

This project relies on:
- **viceroy-lib**: WebAssembly runtime for Fastly Compute (custom fork at https://github.com/KokaKiwi/Viceroy/tree/dev-server)
- **redb**: Embedded database for persistent storage
- **axum**: Web framework for both servers
- **tower**: Middleware and service abstractions
- **OpenTelemetry**: Observability and tracing

## License

See the workspace root [LICENSE.md](../../LICENSE.md) file.

## Related Projects

- [Viceroy](https://github.com/fastly/Viceroy): The official Fastly Compute CLI tool
- [fastly-demo-server](../demo-server/): Example Compute application demonstrating store usage
- [Fastly Compute](https://www.fastly.com/products/edge-compute): The production platform
