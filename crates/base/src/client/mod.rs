use std::time::{SystemTime, UNIX_EPOCH};

use tonic::transport::Channel;
use tucana::{
    aquila::{
        RuntimeStatusUpdateRequest, runtime_status_service_client::RuntimeStatusServiceClient,
        runtime_status_update_request::Status,
    },
    shared::{AdapterConfiguration, AdapterRuntimeStatus, RuntimeFeature},
};

struct DracoRuntimeStatusService {
    channel: Channel,
    identifier: String,
    features: Vec<RuntimeFeature>,
    configs: Vec<AdapterConfiguration>,
}

impl DracoRuntimeStatusService {
    fn new(
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

    async fn add_config(&mut self, feat: RuntimeFeature) {
        self.features.push(feat);
    }

    async fn update_runtime_status_by_status(
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
