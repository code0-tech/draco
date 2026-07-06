use http_body_util::Full;
use hyper::{
    Response, StatusCode,
    body::Bytes,
    header::{HeaderName, HeaderValue},
};
use std::collections::HashMap;
use std::sync::Arc;
use tucana::shared::{
    Struct, ValidationFlow, Value,
    value::Kind::{self, StructValue},
};

use crate::content_type;
use base::store::FlowExecutionResult;

pub async fn flow_execution_to_http_response(
    flow: ValidationFlow,
    input: Value,
    store: Arc<base::store::AdapterStore>,
) -> Response<Full<Bytes>> {
    let result = store.execute_flow_with_emitter(flow, Some(input)).await;
    flow_execution_result_to_http_response(result)
}

fn flow_execution_result_to_http_response(result: FlowExecutionResult) -> Response<Full<Bytes>> {
    match result {
        FlowExecutionResult::Ongoing(result) => {
            log::debug!("Received first ongoing response from emitter");
            value_to_http_response(result)
        }
        FlowExecutionResult::Failed => {
            log::error!("Flow execution failed event received from emitter");
            error_to_http_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
        FlowExecutionResult::FinishedWithoutOngoing => Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Full::new(Bytes::new()))
            .unwrap(),
        FlowExecutionResult::TransportError => {
            log::error!("Flow execution transport error");
            error_to_http_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
    }
}

pub fn error_to_http_response(status: StatusCode, msg: &str) -> Response<Full<Bytes>> {
    let body = format!(r#"{{"error": "{}"}}"#, msg);
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

pub fn value_to_http_response(value: Value) -> Response<Full<Bytes>> {
    let Value {
        kind: Some(StructValue(Struct { fields })),
    } = value
    else {
        return error_to_http_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Flow result was not a struct",
        );
    };

    let Some(headers_val) = fields.get("headers") else {
        return error_to_http_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Flow result missing the field: headers",
        );
    };
    let Some(status_code_val) = fields.get("http_status_code") else {
        return error_to_http_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Flow result missing the field: http_status_code",
        );
    };
    let Some(payload_val) = fields.get("payload") else {
        return error_to_http_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Flow result missing the field: payload",
        );
    };

    let Value {
        kind: Some(Kind::StructValue(Struct {
            fields: header_fields,
        })),
    } = headers_val
    else {
        return error_to_http_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "headers was not a list of header entries",
        );
    };

    let mut http_headers: HashMap<String, String> = header_fields
        .iter()
        .filter_map(|(k, v)| {
            if let Value {
                kind: Some(Kind::StringValue(x)),
            } = v
            {
                Some((k.clone(), x.clone()))
            } else {
                None
            }
        })
        .collect();

    if find_header_value_case_insensitive(&http_headers, "content-type").is_none() {
        http_headers.insert("content-type".to_string(), "application/json".to_string());
    }

    let Some(Kind::NumberValue(code)) = status_code_val.kind else {
        return error_to_http_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "status_code was not a number",
        );
    };

    let content_type_header = find_header_value_case_insensitive(&http_headers, "content-type");
    let encoded_body = match content_type::encode_body(content_type_header, payload_val.clone()) {
        Ok(body) => body,
        Err(err) => {
            log::error!("Failed to encode response payload: {}", err);
            return error_to_http_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to encode response payload",
            );
        }
    };

    let http_code = match code.number {
        Some(num) => match num {
            tucana::shared::number_value::Number::Integer(int) => int as u16,
            tucana::shared::number_value::Number::Float(float) => float as u16,
        },
        None => {
            return error_to_http_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Flow execution failed",
            );
        }
    };

    let status = StatusCode::from_u16(http_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    create_http_response(status, http_headers, encoded_body)
}

fn find_header_value_case_insensitive<'a>(
    headers: &'a HashMap<String, String>,
    key: &str,
) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

fn create_http_response(
    status: StatusCode,
    headers: HashMap<String, String>,
    body: Vec<u8>,
) -> Response<Full<Bytes>> {
    let mut builder = Response::builder().status(status);

    {
        let h = builder.headers_mut().unwrap();
        for (k, v) in headers {
            let name = match HeaderName::from_bytes(k.as_bytes()) {
                Ok(n) => n,
                Err(_) => {
                    log::warn!("Dropping invalid header name: {}", k);
                    continue;
                }
            };

            let value = match HeaderValue::from_str(&v) {
                Ok(v) => v,
                Err(_) => {
                    log::warn!("Dropping invalid header value for {}: {:?}", k, v);
                    continue;
                }
            };

            h.insert(name, value);
        }
    }

    builder.body(Full::new(Bytes::from(body))).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use tucana::shared::{NumberValue, number_value};

    #[tokio::test]
    async fn response_signal_content_is_used() {
        let response_value = Value {
            kind: Some(Kind::StructValue(Struct {
                fields: [
                    (
                        "headers".to_string(),
                        Value {
                            kind: Some(Kind::StructValue(Struct {
                                fields: [
                                    (
                                        "content-type".to_string(),
                                        Value {
                                            kind: Some(Kind::StringValue("text/plain".to_string())),
                                        },
                                    ),
                                    (
                                        "x-flow-response".to_string(),
                                        Value {
                                            kind: Some(Kind::StringValue("matched".to_string())),
                                        },
                                    ),
                                ]
                                .into_iter()
                                .collect(),
                            })),
                        },
                    ),
                    (
                        "http_status_code".to_string(),
                        Value {
                            kind: Some(Kind::NumberValue(NumberValue {
                                number: Some(number_value::Number::Integer(201)),
                            })),
                        },
                    ),
                    (
                        "payload".to_string(),
                        Value {
                            kind: Some(Kind::StringValue("response body".to_string())),
                        },
                    ),
                ]
                .into_iter()
                .collect(),
            })),
        };

        let response =
            flow_execution_result_to_http_response(FlowExecutionResult::Ongoing(response_value));

        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(response.headers()["content-type"], "text/plain");
        assert_eq!(response.headers()["x-flow-response"], "matched");
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            "response body"
        );
    }

    #[tokio::test]
    async fn finished_flow_without_response_returns_no_content() {
        let response =
            flow_execution_result_to_http_response(FlowExecutionResult::FinishedWithoutOngoing);

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .is_empty()
        );
    }
}
