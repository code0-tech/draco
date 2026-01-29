use base::traits::LoadConfig;
use code0_flow::flow_config::env_with_default;


#[derive(Clone)]
pub struct HttpServerConfig {
    pub port: u16,
    pub host: String,
}

impl LoadConfig for HttpServerConfig {
    fn load() -> Self {
        Self {
            port: env_with_default("HTTP_SERVER_PORT", 8080),
            host: env_with_default("HTTP_SERVER_HOST", String::from("127.0.0.1")),
        }
    }
}
