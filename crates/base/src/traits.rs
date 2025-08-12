use crate::runner::ServerContext;
use async_trait::async_trait;
use tucana::shared::ValidationFlow;

/*
 * These are the traits the user must implement to build a server.
 */

/// A trait for loading your app’s configuration.
pub trait LoadConfig: Sized + Clone + Send + Sync + 'static {
    /// Load your concrete config (from env, files, etc.).
    fn load() -> Self;
}

/// The lifecycle your server must implement.
#[async_trait]
pub trait Server<C: LoadConfig>: Send + Sync + 'static {
    /// Called once at startup.
    async fn init(&mut self, ctx: &ServerContext<C>) -> anyhow::Result<()>;

    /// The “serve forever” loop.
    async fn run(&mut self, ctx: &ServerContext<C>) -> anyhow::Result<()>;

    /// Called on shutdown signal.
    async fn shutdown(&mut self, ctx: &ServerContext<C>) -> anyhow::Result<()>;
}

pub trait IdentifiableFlow {
    fn identify(&self, flow: &ValidationFlow) -> bool;
}
