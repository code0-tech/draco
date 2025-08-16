//! Core traits that define the contract for building servers with the Draco framework.
//!
//! This module contains the fundamental traits that users must implement to create
//! a functioning server application. The traits follow a clear lifecycle pattern
//! and provide hooks for configuration loading, initialization, execution, and cleanup.

use crate::runner::ServerContext;
use async_trait::async_trait;
use tucana::shared::ValidationFlow;

/// A trait for loading and managing your application's configuration.
///
/// This trait defines how your server loads its configuration data, whether from
/// environment variables, configuration files, command line arguments, or other sources.
///
/// # Example
///
/// ```rust
/// #[derive(Clone)]
/// struct MyConfig {
///     port: u16,
///     database_url: String,
/// }
///
/// impl LoadConfig for MyConfig {
///     fn load() -> Self {
///         Self {
///             port: std::env::var("PORT")
///                 .unwrap_or_else(|_| "8080".to_string())
///                 .parse()
///                 .expect("Invalid port"),
///             database_url: std::env::var("DATABASE_URL")
///                 .expect("DATABASE_URL must be set"),
///         }
///     }
/// }
/// ```
pub trait LoadConfig: Sized + Clone + Send + Sync + 'static {
    /// Load your application's configuration from external sources.
    ///
    /// This method is called once during server startup and should gather
    /// all necessary configuration data from environment variables, files,
    /// databases, or other sources.
    ///
    /// # Returns
    ///
    /// Returns an instance of your configuration type with all values loaded
    /// and validated.
    ///
    /// # Panics
    ///
    /// This method may panic if required configuration is missing or invalid,
    /// as configuration errors are typically fatal and should prevent startup.
    fn load() -> Self;
}

/// The main server lifecycle trait that defines the complete execution flow.
///
/// This trait represents the core contract for any server implementation in the
/// Draco framework. It provides three distinct phases of server operation:
/// initialization, execution, and shutdown.
///
/// The server lifecycle follows this pattern:
/// 1. `init()` - Perform one-time setup and initialization
/// 2. `run()` - Execute the main server loop (typically runs indefinitely)
/// 3. `shutdown()` - Clean up resources and perform graceful shutdown
///
/// # Example
///
/// ```rust
/// struct MyServer {
///     // Server state fields
/// }
///
/// #[async_trait]
/// impl Server<MyConfig> for MyServer {
///     async fn init(&mut self, ctx: &ServerContext<MyConfig>) -> anyhow::Result<()> {
///         // Initialize database connections, load resources, etc.
///         Ok(())
///     }
///
///     async fn run(&mut self, ctx: &ServerContext<MyConfig>) -> anyhow::Result<()> {
///         // Main server loop - handle requests, process events, etc.
///         loop {
///             // Server logic here
///         }
///     }
///
///     async fn shutdown(&mut self, ctx: &ServerContext<MyConfig>) -> anyhow::Result<()> {
///         // Close connections, save state, cleanup resources
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait Server<C: LoadConfig>: Send + Sync + 'static {
    /// Initialize the server and perform one-time setup operations.
    ///
    /// This method is called once during server startup, after configuration
    /// has been loaded but before the main server loop begins. Use this method
    /// to perform expensive initialization operations such as:
    ///
    /// - Establishing database connections
    /// - Loading static resources or caches
    /// - Validating external dependencies
    /// - Setting up monitoring or logging systems
    ///
    /// # Parameters
    ///
    /// - `ctx`: The server context containing configuration and runtime information
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful initialization, or an error if initialization
    /// fails. Initialization errors will prevent the server from starting.
    ///
    /// # Errors
    ///
    /// This method should return an error if any critical initialization step fails,
    /// such as inability to connect to required external services.
    async fn init(&mut self, ctx: &ServerContext<C>) -> anyhow::Result<()>;

    /// Execute the main server loop.
    ///
    /// This method contains the core server logic and typically runs indefinitely
    /// until a shutdown signal is received. Common patterns include:
    ///
    /// - HTTP/gRPC server request handling loops
    /// - Message queue consumers
    /// - Event processing loops
    /// - Periodic task execution
    ///
    /// The method should be designed to run continuously and handle graceful
    /// shutdown when interrupted by external signals.
    ///
    /// # Parameters
    ///
    /// - `ctx`: The server context containing configuration and runtime information
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` when the server shuts down gracefully, or an error if
    /// a fatal error occurs during execution.
    ///
    /// # Errors
    ///
    /// This method should return an error only for fatal conditions that require
    /// immediate server termination. Recoverable errors should be handled internally.
    async fn run(&mut self, ctx: &ServerContext<C>) -> anyhow::Result<()>;

    /// Perform graceful shutdown and cleanup operations.
    ///
    /// This method is called when the server receives a shutdown signal (such as
    /// SIGTERM or SIGINT). It should perform cleanup operations to ensure a
    /// graceful shutdown:
    ///
    /// - Close database connections and network sockets
    /// - Flush pending writes or queued operations
    /// - Save application state if necessary
    /// - Release system resources
    ///
    /// The shutdown process should complete within a reasonable timeframe to
    /// avoid being forcefully terminated.
    ///
    /// # Parameters
    ///
    /// - `ctx`: The server context containing configuration and runtime information
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on successful shutdown, or an error if cleanup operations fail.
    /// Shutdown errors are typically logged but don't prevent process termination.
    ///
    /// # Errors
    ///
    /// This method should return an error if critical cleanup operations fail,
    /// though the server will still terminate regardless of the result.
    async fn shutdown(&mut self, ctx: &ServerContext<C>) -> anyhow::Result<()>;
}

/// A trait for identifying and matching validation flows.
///
/// This trait provides a mechanism to identify whether a given validation flow
/// matches specific criteria. It's typically used in systems that need to route
/// or process different types of validation flows based on their characteristics.
pub trait IdentifiableFlow {
    /// Determine if this identifier matches the given validation flow.
    ///
    /// This method examines a validation flow and returns whether it matches
    /// the criteria defined by this identifier. The matching logic is entirely
    /// implementation-dependent and can be based on any properties of the flow.
    ///
    /// # Parameters
    ///
    /// - `flow`: The validation flow to examine
    ///
    /// # Returns
    ///
    /// Returns `true` if the flow matches this identifier's criteria,
    /// `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let matcher = MyFlowMatcher::new();
    /// let flow = ValidationFlow { /* ... */ };
    ///
    /// if matcher.identify(&flow) {
    ///     // Handle this specific type of flow
    /// }
    /// ```
    fn identify(&self, flow: &ValidationFlow) -> bool;
}
