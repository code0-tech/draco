# Adapter Creation Guide

A quick guide to building custom adapter servers for the Draco framework.

## What is an Adapter?

An adapter is a server that:
- Listens for requests (HTTP, gRPC, etc.)
- Matches requests to validation flows
- Executes flows and returns results

## Quick Start

### 1. Create Your Project Structure
```
my-adapter/
├── Cargo.toml
└── src/
    └── main.rs
```

### 2. Add Dependencies
Add these to your `Cargo.toml`:
- `base` - Core Draco functionality
- `tokio` - Async runtime
- `async-trait` - Async trait support
- Your protocol library (e.g., `hyper` for HTTP)

### 3. Main Entry Point

```rust
#[tokio::main]
async fn main() {
    let server = MyAdapter::new();
    let runner = ServerRunner::new(server).await.unwrap();
    runner.serve().await.unwrap();
}
```

### 4. Configuration Struct

```rust
#[derive(Clone)]
struct MyAdapterConfig {
    port: u16,
    // Add other settings
}

impl LoadConfig for MyAdapterConfig {
    fn load() -> Self {
        Self {
            port: env_with_default("MY_ADAPTER_PORT", 8080),
            // Load other settings from environment
        }
    }
}
```

### 5. Server Implementation

```rust
struct MyAdapter {
    // Your protocol server instance
    protocol_server: Option<MyProtocolServer>,
}

#[async_trait]
impl ServerTrait<MyAdapterConfig> for MyAdapter {
    async fn init(&mut self, ctx: &ServerContext<MyAdapterConfig>) -> anyhow::Result<()> {
        // Initialize your protocol server with config
        self.protocol_server = Some(MyProtocolServer::new(ctx.server_config.port));
        Ok(())
    }

    async fn run(&mut self, ctx: &ServerContext<MyAdapterConfig>) -> anyhow::Result<()> {
        if let Some(server) = &mut self.protocol_server {
            // Register request handler
            server.register_handler({
                let store = Arc::clone(&ctx.adapter_store);
                move |request| {
                    let store = Arc::clone(&store);
                    async move {
                        handle_request(request, store).await
                    }
                }
            });

            // Start your protocol server
            server.start().await;
        }
        Ok(())
    }

    async fn shutdown(&mut self, _ctx: &ServerContext<MyAdapterConfig>) -> anyhow::Result<()> {
        if let Some(server) = &self.protocol_server {
            server.shutdown();
        }
        Ok(())
    }
}
```

### 6. Flow Matcher

```rust
struct MyRequestMatcher {
    // Store request details that help identify flows
    path: String,
    method: String,
}

impl IdentifiableFlow for MyRequestMatcher {
    fn identify(&self, flow: &ValidationFlow) -> bool {
        // Extract expected values from flow settings
        let expected_path = extract_flow_setting_field(
            &flow.settings,
            "REQUEST_PATH",
            "path"
        );

        // Check if this request matches the flow
        match expected_path.as_deref() {
            Some(pattern) => {
                // Use regex or simple matching
                regex::Regex::new(pattern)
                    .map(|r| r.is_match(&self.path))
                    .unwrap_or(false)
            }
            None => false,
        }
    }
}
```

### 7. Request Handler

```rust
async fn handle_request(request: MyRequest, store: Arc<AdapterStore>) -> MyResponse {
    // Create pattern for flow matching
    let pattern = format!("*.*.{}.{}.{}",
                         "MY_PROTOCOL",
                         request.host,
                         request.method);

    // Create matcher with request details
    let matcher = MyRequestMatcher {
        path: request.path.clone(),
        method: request.method.clone(),
    };

    // Find matching flows
    match store.get_possible_flow_match(pattern, matcher).await {
        FlowIdenfiyResult::Single(flow) => {
            // Execute the flow
            match store.validate_and_execute_flow(flow, request.body).await {
                Some(result) => {
                    // Convert result to your protocol response format
                    create_success_response(result)
                }
                None => {
                    create_error_response("Flow execution failed")
                }
            }
        }
        _ => {
            create_error_response("No matching flow found")
        }
    }
}
```
