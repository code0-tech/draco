use base::traits::LoadConfig;
use code0_flow::flow_config::env_with_default;

#[derive(Clone)]
pub struct HttpServerConfig {
    pub port: u16,
    pub external_port: u16,
    pub host: String,
    pub external_host: String,
}

impl LoadConfig for HttpServerConfig {
    fn load() -> Self {
        let port = env_with_default("HTTP_SERVER_PORT", 8080);
        let host = env_with_default("HTTP_SERVER_HOST", String::from("127.0.0.1"));

        Self {
            host: host.clone(),
            port,
            external_port: env_with_default("EXTERNAL_HTTP_SERVER_PORT", port),
            external_host: env_with_default("EXTERNAL_HTTP_SERVER_HOST", host),
        }
    }
}
