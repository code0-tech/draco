use crate::traits::IdentifiableFlow;
use async_nats::jetstream::kv::Config;
use futures_lite::StreamExt;
use prost::Message;
use tucana::shared::{
    ExecutionFlow, Struct, ValidationFlow, Value,
    value::Kind::{self, StructValue},
};

const EMITTER_TOPIC_PREFIX: &str = "runtime.emitter";
const EMITTER_WAIT_TIMEOUT_SECONDS: u64 = 30;

pub struct AdapterStore {
    client: async_nats::Client,
    kv: async_nats::jetstream::kv::Store,
}

pub enum FlowIdentifyResult {
    None,
    Single(ValidationFlow),
    Multiple(Vec<ValidationFlow>),
}

pub enum FlowExecutionResult {
    Ongoing(Value),
    Failed,
    FinishedWithoutOngoing,
    TransportError,
}

impl AdapterStore {
    pub async fn from_url(url: String, bucket: String) -> Self {
        let client = match async_nats::connect(url).await {
            Ok(client) => {
                log::info!("Successfully connected to NATS");
                client
            }
            Err(err) => panic!("Failed to connect to NATS server: {:?}", err),
        };

        let stream = async_nats::jetstream::new(client.clone());

        match stream
            .create_key_value(Config {
                bucket: bucket.clone(),
                ..Default::default()
            })
            .await
        {
            Ok(_) => {
                log::info!("Successfully created NATS bucket/bucket already exists");
            }
            Err(err) => panic!("Failed to create NATS bucket: {:?}", err),
        }

        let kv = match stream.get_key_value(bucket).await {
            Ok(kv) => {
                log::info!("Successfully got NATS bucket");
                kv
            }
            Err(err) => panic!("Failed to get key-value store: {}", err),
        };

        Self { client, kv }
    }

    /// get_possible_flow_matches
    ///
    /// This function will take a key that one or more keys of a flow.
    /// It will then loop over every value received from the key and return all flows that matched the IdentifiedFlow trait.
    ///
    /// Arguments:
    /// - pattern: The key to get possible flow matches. For example, a REST Flow is never completely identifiable through a single key because the URL is dynamic and wherefore a regex is needed to be applied to the url making it impossible to include the entire URL in the key. In this case the key just reduces the amount of flows that can be a possible match.
    /// - id: The identifier to use for identifying the possible matches. Its just a fine grain identifier that can be used to identify the possible matches. For a REST Flow this will be the regex matcher, for a CRON Flow the trait just return true every time.
    ///
    /// Returns:
    /// - FlowIdenfiyResult: The result of the flow identification process. This can be one of the following:
    ///     - None: No flows matched the identifier.
    ///     - Single(ValidationFlow): A single flow matched the identifier.
    ///     - Multiple(Vec<ValidationFlow>): Multiple flows matched the identifier.
    ///
    /// None is always bad, but as always this depends on the Adapter type.
    /// For example:
    /// REST will have only one match, if multiple matches are found it means the regex is not correct.
    /// CRON can have multiple matches, because multiple flows can have the same CRON expression.
    pub async fn get_possible_flow_match<I: IdentifiableFlow>(
        &self,
        pattern: String,
        id: I,
    ) -> FlowIdentifyResult {
        let mut collector = Vec::new();
        let mut keys = match self.kv.keys().await {
            Ok(keys) => keys.boxed(),
            Err(err) => {
                log::error!("Failed to get keys: {}", err);
                return FlowIdentifyResult::None;
            }
        };

        while let Ok(Some(key)) = keys.try_next().await {
            if !Self::is_matching_key(&pattern, &key) {
                continue;
            }

            if let Ok(Some(bytes)) = self.kv.get(key).await {
                let decoded_flow = ValidationFlow::decode(bytes);
                if let Ok(flow) = decoded_flow
                    && id.identify(&flow)
                {
                    collector.push(flow.clone());
                };
            }
        }

        match collector.len() {
            0 => FlowIdentifyResult::None,
            1 => FlowIdentifyResult::Single(collector[0].clone()),
            _ => FlowIdentifyResult::Multiple(collector),
        }
    }

    /// validate_and_execute_flow
    ///
    /// This function will validate the flow. If the flow is valid, it will execute (send the flow to the execution and wait for a/multiple result/s) the flow.
    ///
    /// Arguments:
    /// - flow: The flow to be validated and executed.
    /// - input_value: The input value to be used for the flow execution.
    pub async fn validate_and_execute_flow(
        &self,
        flow: ValidationFlow,
        input_value: Option<Value>,
    ) -> Option<Value> {
        match self.execute_flow_with_emitter(flow, input_value).await {
            FlowExecutionResult::Ongoing(value) => Some(value),
            FlowExecutionResult::Failed
            | FlowExecutionResult::FinishedWithoutOngoing
            | FlowExecutionResult::TransportError => None,
        }
    }

    pub async fn execute_flow_with_emitter(
        &self,
        flow: ValidationFlow,
        input_value: Option<Value>,
    ) -> FlowExecutionResult {
        // TODO: Replace body vaidation with triangulus when its ready
        let execution_id = uuid::Uuid::new_v4().to_string();
        let flow_id = flow.flow_id;
        let execution_flow: ExecutionFlow =
            Self::convert_validation_flow(flow, input_value.clone());
        let bytes = execution_flow.encode_to_vec();
        let execution_topic = format!("execution.{}", execution_id);
        let emitter_topic = format!("{}.{}", EMITTER_TOPIC_PREFIX, execution_id);

        log::info!(
            "Requesting execution of flow {} with execution id {}",
            flow_id,
            execution_id
        );
        log::debug!(
            "Flow Input for Execution ({}) is {:?}",
            execution_id,
            input_value
        );

        let mut subscriber = match self.client.subscribe(emitter_topic.clone()).await {
            Ok(subscriber) => subscriber,
            Err(err) => {
                log::error!(
                    "Failed to subscribe to emitter topic '{}' for flow {}: {:?}",
                    emitter_topic,
                    flow_id,
                    err
                );
                return FlowExecutionResult::TransportError;
            }
        };

        if let Err(err) = self
            .client
            .publish(execution_topic.clone(), bytes.into())
            .await
        {
            log::error!(
                "Failed to publish flow {} to execution topic '{}': {:?}",
                flow_id,
                execution_topic,
                err
            );
            return FlowExecutionResult::TransportError;
        }

        loop {
            let next_message = tokio::time::timeout(
                std::time::Duration::from_secs(EMITTER_WAIT_TIMEOUT_SECONDS),
                subscriber.next(),
            )
            .await;

            let message = match next_message {
                Ok(Some(message)) => message,
                Ok(None) => {
                    log::error!(
                        "Emitter subscription '{}' closed before execution completed",
                        emitter_topic
                    );
                    return FlowExecutionResult::TransportError;
                }
                Err(_) => {
                    log::error!(
                        "Timed out waiting for emitter events on '{}' after {}s",
                        emitter_topic,
                        EMITTER_WAIT_TIMEOUT_SECONDS
                    );
                    return FlowExecutionResult::TransportError;
                }
            };

            let Some((emit_type, payload)) = Self::decode_emit_message(message.payload.as_ref())
            else {
                continue;
            };

            match emit_type.as_str() {
                "starting" => {}
                "ongoing" => return FlowExecutionResult::Ongoing(payload),
                "failed" => return FlowExecutionResult::Failed,
                "finished" => return FlowExecutionResult::FinishedWithoutOngoing,
                other => {
                    log::warn!(
                        "Received unknown emitter event '{}' on '{}'",
                        other,
                        emitter_topic
                    );
                }
            }
        }
    }

    fn convert_validation_flow(flow: ValidationFlow, input_value: Option<Value>) -> ExecutionFlow {
        ExecutionFlow {
            flow_id: flow.flow_id,
            starting_node_id: flow.starting_node_id,
            input_value,
            node_functions: flow.node_functions,
            project_id: flow.project_id,
        }
    }

    fn is_matching_key(pattern: &String, key: &String) -> bool {
        let split_pattern = pattern.split(".");
        let split_key = key.split(".").collect::<Vec<&str>>();
        let zip = split_pattern.into_iter().zip(split_key);

        for (pattern_part, key_part) in zip {
            if pattern_part == "*" {
                continue;
            }

            if pattern_part != key_part {
                return false;
            }
        }
        true
    }

    fn decode_emit_message(bytes: &[u8]) -> Option<(String, Value)> {
        let decoded = match Value::decode(bytes) {
            Ok(value) => value,
            Err(err) => {
                log::error!("Failed to decode emitter payload: {:?}", err);
                return None;
            }
        };

        let Value {
            kind: Some(StructValue(Struct { fields })),
        } = decoded
        else {
            log::warn!("Emitter payload was not a struct value");
            return None;
        };

        let Some(emit_type) = fields.get("emit_type") else {
            log::warn!("Emitter payload is missing 'emit_type'");
            return None;
        };
        let Some(payload) = fields.get("payload") else {
            log::warn!("Emitter payload is missing 'payload'");
            return None;
        };

        let Some(Kind::StringValue(emit_type_str)) = emit_type.kind.as_ref() else {
            log::warn!("Emitter payload field 'emit_type' was not a string");
            return None;
        };

        Some((emit_type_str.clone(), payload.clone()))
    }
}
