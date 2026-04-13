use hyper::{
    HeaderMap,
    header::{CONTENT_TYPE, HeaderValue},
};
use tucana::shared::{
    Value, number_value,
    value::Kind::{self, StringValue},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BodyFormat {
    Json,
    TextPlain,
    Unknown,
}

#[derive(Debug)]
pub enum BodyParseError {
    UnsupportedContentType { observed: String },
    InvalidUtf8(std::str::Utf8Error),
    InvalidJson(serde_json::Error),
}

impl std::fmt::Display for BodyParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContentType { observed } => {
                write!(f, "unsupported content type: {}", observed)
            }
            Self::InvalidUtf8(err) => write!(f, "invalid UTF-8 body: {}", err),
            Self::InvalidJson(err) => write!(f, "invalid JSON body: {}", err),
        }
    }
}

impl std::error::Error for BodyParseError {}

#[derive(Debug)]
pub enum BodyEncodeError {
    UnsupportedContentType { observed: String },
    InvalidJson(serde_json::Error),
}

impl std::fmt::Display for BodyEncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedContentType { observed } => {
                write!(f, "unsupported content type: {}", observed)
            }
            Self::InvalidJson(err) => write!(f, "failed to encode JSON body: {}", err),
        }
    }
}

impl std::error::Error for BodyEncodeError {}

pub fn parse_body_from_headers(
    headers: &HeaderMap<HeaderValue>,
    body: &[u8],
) -> Result<Option<Value>, BodyParseError> {
    parse_body(get_content_type(headers), body)
}

pub fn parse_body(
    content_type: Option<&str>,
    body: &[u8],
) -> Result<Option<Value>, BodyParseError> {
    if body.is_empty() {
        return Ok(None);
    }

    match classify_content_type(content_type) {
        BodyFormat::Json => parse_json_body(body),
        BodyFormat::TextPlain => parse_text_body(body),
        BodyFormat::Unknown => {
            // If there is no content type
            if content_type.is_none()
                && let Ok(value) = parse_text_body(body)
            {
                return Ok(value);
            }

            Err(BodyParseError::UnsupportedContentType {
                observed: content_type.unwrap_or("<missing>").to_string(),
            })
        }
    }
}

pub fn encode_body(content_type: Option<&str>, value: Value) -> Result<Vec<u8>, BodyEncodeError> {
    match classify_content_type(content_type) {
        BodyFormat::Json => encode_json_body(value),
        BodyFormat::TextPlain => encode_text_body(value),
        BodyFormat::Unknown => {
            // Missing content type falls back to JSON.
            if content_type.is_none() {
                return encode_json_body(value);
            }

            Err(BodyEncodeError::UnsupportedContentType {
                observed: content_type.unwrap_or("<missing>").to_string(),
            })
        }
    }
}

pub fn classify_content_type(content_type: Option<&str>) -> BodyFormat {
    let Some(raw) = content_type else {
        return BodyFormat::Unknown;
    };

    let essence = raw
        .split(';')
        .next()
        .unwrap_or(raw)
        .trim()
        .to_ascii_lowercase();

    if essence == "application/json" || essence.ends_with("+json") {
        return BodyFormat::Json;
    }

    if essence == "text/plain" {
        return BodyFormat::TextPlain;
    }

    BodyFormat::Unknown
}

fn parse_json_body(body: &[u8]) -> Result<Option<Value>, BodyParseError> {
    let json_value =
        serde_json::from_slice::<serde_json::Value>(body).map_err(BodyParseError::InvalidJson)?;
    Ok(Some(tucana::shared::helper::value::from_json_value(
        json_value,
    )))
}

fn parse_text_body(body: &[u8]) -> Result<Option<Value>, BodyParseError> {
    let text = std::str::from_utf8(body).map_err(BodyParseError::InvalidUtf8)?;
    Ok(Some(Value {
        kind: Some(StringValue(text.to_string())),
    }))
}

fn encode_json_body(value: Value) -> Result<Vec<u8>, BodyEncodeError> {
    let json_val = tucana::shared::helper::value::to_json_value(value);
    serde_json::to_vec_pretty(&json_val).map_err(BodyEncodeError::InvalidJson)
}

fn encode_text_body(value: Value) -> Result<Vec<u8>, BodyEncodeError> {
    if let Some(text) = scalar_to_text(&value) {
        return Ok(text.into_bytes());
    }

    // For lists/objects, return valid JSON text as the plain-text body.
    encode_json_body(value)
}

fn scalar_to_text(value: &Value) -> Option<String> {
    match value.kind.as_ref() {
        Some(Kind::NullValue(_)) | None => Some("null".to_string()),
        Some(Kind::BoolValue(v)) => Some(v.to_string()),
        Some(Kind::StringValue(v)) => Some(v.clone()),
        Some(Kind::NumberValue(v)) => match v.number.as_ref() {
            Some(number_value::Number::Integer(i)) => Some(i.to_string()),
            Some(number_value::Number::Float(f)) => Some(f.to_string()),
            None => Some("null".to_string()),
        },
        _ => None,
    }
}

fn get_content_type(headers: &HeaderMap<HeaderValue>) -> Option<&str> {
    headers.get(CONTENT_TYPE).and_then(|h| h.to_str().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tucana::shared::{NumberValue, Struct, Value};

    #[test]
    fn classify_json_content_type_with_charset() {
        let format = classify_content_type(Some("application/json; charset=utf-8"));
        assert_eq!(format, BodyFormat::Json);
    }

    #[test]
    fn classify_vendor_json_content_type() {
        let format = classify_content_type(Some("application/problem+json"));
        assert_eq!(format, BodyFormat::Json);
    }

    #[test]
    fn classify_text_plain_content_type() {
        let format = classify_content_type(Some("text/plain; charset=utf-8"));
        assert_eq!(format, BodyFormat::TextPlain);
    }

    #[test]
    fn parse_json_body_to_struct_value() {
        let body = br#"{"hello":"world","ok":true}"#;
        let parsed = parse_body(Some("application/json"), body).unwrap();

        let Some(Value {
            kind: Some(Kind::StructValue(Struct { fields })),
        }) = parsed
        else {
            panic!("expected struct value");
        };

        assert!(fields.contains_key("hello"));
        assert!(fields.contains_key("ok"));
    }

    #[test]
    fn parse_text_body_to_string_value() {
        let body = b"hello";
        let parsed = parse_body(Some("text/plain"), body).unwrap();

        let Some(Value {
            kind: Some(Kind::StringValue(v)),
        }) = parsed
        else {
            panic!("expected string value");
        };

        assert_eq!(v, "hello");
    }

    #[test]
    fn parse_missing_content_type_best_effort_json() {
        let body = br#"{"name":"draco"}"#;
        let parsed = parse_body(None, body).unwrap();

        let Some(Value {
            kind: Some(Kind::StructValue(_)),
        }) = parsed
        else {
            panic!("expected best-effort JSON parse");
        };
    }

    #[test]
    fn parse_unsupported_content_type_fails() {
        let body = br#"<root />"#;
        let err = parse_body(Some("application/xml"), body).unwrap_err();

        assert!(matches!(err, BodyParseError::UnsupportedContentType { .. }));
    }

    #[test]
    fn encode_json_body_from_struct_value() {
        let value = Value {
            kind: Some(Kind::StructValue(Struct {
                fields: [(
                    "hello".to_string(),
                    Value {
                        kind: Some(Kind::StringValue("world".to_string())),
                    },
                )]
                .into_iter()
                .collect(),
            })),
        };

        let encoded = encode_body(Some("application/json"), value).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(parsed["hello"], "world");
    }

    #[test]
    fn encode_text_body_from_string_value() {
        let value = Value {
            kind: Some(Kind::StringValue("hello".to_string())),
        };

        let encoded = encode_body(Some("text/plain"), value).unwrap();
        assert_eq!(encoded, b"hello".to_vec());
    }

    #[test]
    fn encode_text_body_from_number_value() {
        let value = Value {
            kind: Some(Kind::NumberValue(NumberValue {
                number: Some(number_value::Number::Integer(42)),
            })),
        };

        let encoded = encode_body(Some("text/plain"), value).unwrap();
        assert_eq!(encoded, b"42".to_vec());
    }

    #[test]
    fn encode_text_body_from_struct_value_falls_back_to_json_text() {
        let value = Value {
            kind: Some(Kind::StructValue(Struct {
                fields: [(
                    "answer".to_string(),
                    Value {
                        kind: Some(Kind::NumberValue(NumberValue {
                            number: Some(number_value::Number::Integer(42)),
                        })),
                    },
                )]
                .into_iter()
                .collect(),
            })),
        };

        let encoded = encode_body(Some("text/plain"), value).unwrap();
        let body_text = String::from_utf8(encoded).unwrap();

        assert!(body_text.contains("\"answer\""));
        assert!(body_text.contains("42"));
    }

    #[test]
    fn encode_unknown_content_type_fails() {
        let value = Value {
            kind: Some(Kind::StringValue("x".to_string())),
        };

        let err = encode_body(Some("application/xml"), value).unwrap_err();
        assert!(matches!(
            err,
            BodyEncodeError::UnsupportedContentType { .. }
        ));
    }
}
