use crate::Context;
use async_trait::async_trait;
use tucana::shared::{ValidationFlow, Value};

/*
 * These are the traits the user must implement to build a server.
 */

pub trait Request {
    fn to_value(&self) -> Value;
}

/// A trait for loading your app’s configuration.
pub trait LoadConfig: Sized + Clone + Send + Sync + 'static {
    /// Load your concrete config (from env, files, etc.).
    fn load() -> anyhow::Result<Self>;
}

/// The lifecycle your server must implement.
#[async_trait]
pub trait Server<C: LoadConfig>: Send + Sync + 'static {
    /// Called once at startup.
    async fn init(&mut self, ctx: &Context<C>) -> anyhow::Result<()>;

    /// The “serve forever” loop.
    async fn run(&mut self, ctx: &Context<C>) -> anyhow::Result<()>;

    /// Called on shutdown signal.
    async fn shutdown(&mut self, ctx: &Context<C>) -> anyhow::Result<()>;
}

pub trait IdentifiableFlow {
    fn identify(&self, flows: &Vec<ValidationFlow>) -> Option<ValidationFlow>;
}
