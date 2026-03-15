use std::time::{SystemTime, UNIX_EPOCH};

use tokio::time::sleep;
use tonic::transport::{Channel, Endpoint};
use tucana::{
    aquila::{
        RuntimeStatusUpdateRequest, runtime_status_service_client::RuntimeStatusServiceClient,
        runtime_status_update_request::Status,
    },
    shared::{AdapterConfiguration, AdapterRuntimeStatus, RuntimeFeature},
};

pub struct DracoRuntimeStatusService {
    channel: Channel,
    identifier: String,
    features: Vec<RuntimeFeature>,
    configs: Vec<AdapterConfiguration>,
}

const MAX_BACKOFF: u64 = 2000 * 60;
const MAX_RETRIES: i8 = 10;

// Will create a channel and retry if its not possible
pub async fn create_channel_with_retry(channel_name: &str, url: String) -> Channel {
    let mut backoff = 100;
    let mut retries = 0;

    loop {
        let channel = match Endpoint::from_shared(url.clone()) {
            Ok(c) => {
                log::debug!("Creating a new endpoint for the: {} Service", channel_name);
                c.connect_timeout(std::time::Duration::from_secs(2))
                    .timeout(std::time::Duration::from_secs(10))
            }
            Err(err) => {
                panic!(
                    "Cannot create Endpoint for Service: `{}`. Reason: {:?}",
                    channel_name, err
                );
            }
        };

        match channel.connect().await {
            Ok(ch) => {
                return ch;
            }
            Err(err) => {
                log::warn!(
                    "Retry connect to `{}` using url: `{}` failed: {:?}, retrying in {}ms",
                    channel_name,
                    url,
                    err,
                    backoff
                );
                sleep(std::time::Duration::from_millis(backoff)).await;

                backoff = (backoff * 2).min(MAX_BACKOFF);
                retries += 1;

                if retries >= MAX_RETRIES {
                    panic!("Reached max retries to url {}", url)
                }
            }
        }
    }
}
impl DracoRuntimeStatusService {
    pub async fn from_url(
        aquila_url: String,
        identifier: String,
        features: Vec<RuntimeFeature>,
        configs: Vec<AdapterConfiguration>,
    ) -> Self {
        let channel = create_channel_with_retry("Aquila", aquila_url).await;
        Self::new(channel, identifier, features, configs)
    }

    pub fn new(
        channel: Channel,
        identifier: String,
        features: Vec<RuntimeFeature>,
        configs: Vec<AdapterConfiguration>,
    ) -> Self {
        DracoRuntimeStatusService {
            channel,
            identifier,
            features,
            configs,
        }
    }

    pub async fn update_runtime_status_by_status(
        &self,
        status: tucana::shared::adapter_runtime_status::Status,
    ) {
        log::info!("Updating the current Runtime Status!");
        let mut client = RuntimeStatusServiceClient::new(self.channel.clone());

        let now = SystemTime::now();
        let timestamp = match now.duration_since(UNIX_EPOCH) {
            Ok(time) => time.as_secs(),
            Err(err) => {
                log::error!("cannot get current system time: {:?}", err);
                0
            }
        };

        let request = RuntimeStatusUpdateRequest {
            status: Some(Status::AdapterRuntimeStatus(AdapterRuntimeStatus {
                status: status.into(),
                timestamp: timestamp as i64,
                identifier: self.identifier.clone(),
                features: self.features.clone(),
                configurations: self.configs.clone(),
            })),
        };

        match client.update(request).await {
            Ok(response) => {
                log::info!(
                    "Was the update of the RuntimeStatus accepted by Sagittarius? {}",
                    response.into_inner().success
                );
            }
            Err(err) => {
                log::error!("Failed to update RuntimeStatus: {:?}", err);
            }
        }
    }
    async fn update_runtime_status(&self, status: AdapterRuntimeStatus) {
        log::info!("Updating the current Runtime Status!");
        let mut client = RuntimeStatusServiceClient::new(self.channel.clone());

        let request = RuntimeStatusUpdateRequest {
            status: Some(Status::AdapterRuntimeStatus(status)),
        };

        match client.update(request).await {
            Ok(response) => {
                log::info!(
                    "Was the update of the RuntimeStatus accepted by Sagittarius? {}",
                    response.into_inner().success
                );
            }
            Err(err) => {
                log::error!("Failed to update RuntimeStatus: {:?}", err);
            }
        }
    }
}
