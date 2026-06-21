mod input;

use base::store::FlowIdentifyResult;
use http_body_util::{BodyExt, Full};
use hyper::{
    Request, Response, StatusCode,
    body::{Bytes, Incoming},
};
use std::convert::Infallible;
use std::sync::Arc;

use crate::auth::{authenticate_header_name, validate_flow_auth};
use crate::content_type;
use crate::response::{error_to_http_response, flow_execution_to_http_response};
use crate::route::{self, RequestRoute};

pub async fn handle(
    req: Request<Incoming>,
    store: Arc<base::store::AdapterStore>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(str::to_owned);
    let headers = req.headers().clone();

    let body_bytes = match BodyExt::collect(req.into_body()).await {
        Ok(collected) => collected.to_bytes().to_vec(),
        Err(err) => {
            log::error!("Failed to read request body: {}", err);
            return Ok(error_to_http_response(
                StatusCode::BAD_REQUEST,
                "Failed to read request body",
            ));
        }
    };

    let Some(slug) = route::extract_slug_from_path(&path) else {
        return Ok(error_to_http_response(
            StatusCode::BAD_REQUEST,
            "Missing slug in path",
        ));
    };

    let pattern = format!("REST.{}.*", slug);
    let route = RequestRoute {
        url: path.clone(),
        method,
    };

    let response = match store.get_possible_flow_match(pattern, route).await {
        FlowIdentifyResult::Single(flow) => {
            if let Err(err) = validate_flow_auth(&flow, &headers) {
                let mut response = error_to_http_response(err.status_code(), err.message());
                response
                    .headers_mut()
                    .insert(authenticate_header_name(), err.challenge());
                return Ok(response);
            }

            let request_body_value = match parse_request_body(&headers, &body_bytes) {
                Ok(value) => value,
                Err(response) => return Ok(response),
            };

            let input = input::build_flow_input(
                &flow,
                &path,
                query.as_deref(),
                &headers,
                request_body_value,
            );

            flow_execution_to_http_response(flow, input, store).await
        }
        _ => error_to_http_response(StatusCode::NOT_FOUND, "No flow found for path"),
    };

    Ok(response)
}

fn parse_request_body(
    headers: &hyper::HeaderMap<hyper::header::HeaderValue>,
    body_bytes: &[u8],
) -> Result<Option<tucana::shared::Value>, Response<Full<Bytes>>> {
    content_type::parse_body_from_headers(headers, body_bytes).map_err(|err| {
        log::warn!("Failed to parse request body: {}", err);
        let status_code = match err {
            content_type::BodyParseError::UnsupportedContentType { .. } => {
                StatusCode::UNSUPPORTED_MEDIA_TYPE
            }
            _ => StatusCode::BAD_REQUEST,
        };

        error_to_http_response(status_code, &err.to_string())
    })
}
