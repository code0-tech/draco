use code0_flow::flow_config::environment::Environment;
use code0_flow::flow_config::mode::Mode;

/// Service Configuration
/// This configuration holds the setup for every Adapter.
/// If your Adapter needs more configuration, implement the `LoadConfig` trait.
pub struct AdapterConfig {
    /// Service Environment
    pub environment: Environment,

    /// STATIC:
    /// The service will start with no Sagittarius in mind.
    /// No Aquila connection will be established.
    ///
    /// DYNAMIC:
    /// The service will start with Sagittarius in mind.
    /// Aquila connection will be established.
    pub mode: Mode,

    /// NATS URL
    ///
    /// URL of the NATS server to connect to.
    pub nats_url: String,

    /// NATS Bucket
    ///
    /// Name of the NATS bucket to use.
    pub nats_bucket: String,

    /// GRPC Port
    ///
    /// Port on which the adapter's Health Service server will listen.
    pub grpc_port: u16,

    /// GRPC Host
    ///
    /// Host on which the adapter's Health Service server will listen.
    pub grpc_host: String,

    /// Aquila URL
    ///
    /// URL of the Aquila server to connect to.
    pub aquila_url: String,

    /// Definition Path
    ///
    /// Path to the root definition folder.
    pub definition_path: String,

    /// Is Monitored
    ///
    /// If true the Adapter will expose a grpc health service server.
    pub with_health_service: bool,

    /// Variant
    ///
    /// The Variant of Draco. E.g. Http, Cron...
    pub draco_variant: String,
}

impl AdapterConfig {
    pub fn from_env() -> Self {
        let nats_url = code0_flow::flow_config::env_with_default(
            "NATS_URL",
            String::from("nats://localhost:4222"),
        );
        let nats_bucket =
            code0_flow::flow_config::env_with_default("NATS_BUCKET", String::from("flow_store"));
        let grpc_port = code0_flow::flow_config::env_with_default("GRPC_PORT", 50051);
        let grpc_host =
            code0_flow::flow_config::env_with_default("GRPC_HOST", String::from("localhost"));
        let aquila_url = code0_flow::flow_config::env_with_default(
            "AQUILA_URL",
            String::from("grpc://localhost:50051"),
        );

        let environment =
            code0_flow::flow_config::env_with_default("ENVIRONMENT", Environment::Development);
        let mode = code0_flow::flow_config::env_with_default("MODE", Mode::STATIC);
        let definition_path = code0_flow::flow_config::env_with_default(
            "DEFINITION_PATH",
            String::from("./definition"),
        );
        let with_health_service =
            code0_flow::flow_config::env_with_default("WITH_HEALTH_SERVICE", false);

        let draco_variant =
            code0_flow::flow_config::env_with_default("DRACO_VARIANT", String::from("None"));
        Self {
            environment,
            nats_bucket,
            mode,
            nats_url,
            grpc_port,
            grpc_host,
            aquila_url,
            definition_path,
            with_health_service,
            draco_variant,
        }
    }

    pub fn is_static(&self) -> bool {
        self.mode == Mode::STATIC
    }
}
