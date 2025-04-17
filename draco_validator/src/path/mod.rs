pub mod path {
    use tucana::shared::{value::Kind, Value};

    /// Get the Kind at a given path from a Value
    /// Returns None if:
    /// - Path is invalid or doesn't exist in the Value
    /// - Value at the path doesn't have a kind
    /// - Path traversal encounters a non-struct value
    pub fn expect_kind(path: &str, value: &Value) -> Option<Kind> {
        let kind = match &value.kind {
            Some(kind) => kind,
            None => return None,
        };

        let mut items: Vec<&str> = path.split(".").collect();

        if items.is_empty() {
            return None;
        }

        let first = match &items.first() {
            Some(key) => key.to_string().clone(),
            None => return None,
        };

        items.remove(0);

        match kind {
            Kind::StructValue(struct_value) => match struct_value.fields.get(&first) {
                Some(value) => {
                    if items.is_empty() {
                        match &value.kind {
                            Some(kind) => return Some(kind.clone()),
                            None => return None,
                        }
                    } else {
                        return expect_kind(items.join(".").as_str(), value);
                    }
                }
                None => return None,
            },
            _ => return None,
        }
    }

    /// Get a reference to a Value at a given path
    /// Returns None if the path doesn't exist
    pub fn get_value<'a>(path: &str, value: &'a Value) -> Option<&'a Value> {
        let kind = match &value.kind {
            Some(kind) => kind,
            None => return None,
        };

        let mut items: Vec<&str> = path.split(".").collect();
        if items.is_empty() {
            return Some(value);
        }

        let first = items.remove(0);

        match kind {
            Kind::StructValue(struct_value) => {
                let field = struct_value.fields.get(first)?;
                if items.is_empty() {
                    Some(field)
                } else {
                    get_value(&items.join("."), field)
                }
            }
            _ => None,
        }
    }

    /// Check if a path exists in a Value
    pub fn exists_path(path: &str, value: &Value) -> bool {
        get_value(path, value).is_some()
    }

    /// Extract a string value from a path
    pub fn get_string(path: &str, value: &Value) -> Option<String> {
        match expect_kind(path, value)? {
            Kind::StringValue(s) => Some(s),
            _ => None,
        }
    }

    /// Extract a number value from a path
    pub fn get_number(path: &str, value: &Value) -> Option<f64> {
        match expect_kind(path, value)? {
            Kind::NumberValue(n) => Some(n),
            _ => None,
        }
    }

    /// Extract a boolean value from a path
    pub fn get_bool(path: &str, value: &Value) -> Option<bool> {
        match expect_kind(path, value)? {
            Kind::BoolValue(b) => Some(b),
            _ => None,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::HashMap;

    use tucana::shared::Value;

    use crate::path::path::{exists_path, expect_kind};

    #[test]
    fn test_expect_none() {
        let value = Value {
            kind: Some(tucana::shared::value::Kind::StructValue(
                tucana::shared::Struct {
                    fields: HashMap::from([
                        (
                            "name".to_string(),
                            Value {
                                kind: Some(tucana::shared::value::Kind::StringValue(
                                    "John".to_string(),
                                )),
                            },
                        ),
                        (
                            "age".to_string(),
                            Value {
                                kind: Some(tucana::shared::value::Kind::NumberValue(30.0)),
                            },
                        ),
                    ]),
                },
            )),
        };

        assert_eq!(expect_kind(".", &value), None);
        assert_eq!(expect_kind("", &value), None);
    }

    #[test]
    fn test_expect_kind() {
        let value = Value {
            kind: Some(tucana::shared::value::Kind::StructValue(
                tucana::shared::Struct {
                    fields: HashMap::from([
                        (
                            "name".to_string(),
                            Value {
                                kind: Some(tucana::shared::value::Kind::StringValue(
                                    "John".to_string(),
                                )),
                            },
                        ),
                        (
                            "age".to_string(),
                            Value {
                                kind: Some(tucana::shared::value::Kind::NumberValue(30.0)),
                            },
                        ),
                    ]),
                },
            )),
        };
        assert_eq!(
            expect_kind("name", &value),
            Some(tucana::shared::value::Kind::StringValue("John".to_string()))
        );
        assert_eq!(
            expect_kind("age", &value),
            Some(tucana::shared::value::Kind::NumberValue(30.0))
        );
        assert_eq!(expect_kind("address", &value), None);
    }

    #[test]
    fn test_expect_kind_nested() {
        let value = Value {
            kind: Some(tucana::shared::value::Kind::StructValue(
                tucana::shared::Struct {
                    fields: HashMap::from([
                        (
                            "name".to_string(),
                            Value {
                                kind: Some(tucana::shared::value::Kind::StringValue(
                                    "John".to_string(),
                                )),
                            },
                        ),
                        (
                            "address".to_string(),
                            Value {
                                kind: Some(tucana::shared::value::Kind::StructValue(
                                    tucana::shared::Struct {
                                        fields: HashMap::from([
                                            (
                                                "street".to_string(),
                                                Value {
                                                    kind: Some(
                                                        tucana::shared::value::Kind::StringValue(
                                                            "123 Main St".to_string(),
                                                        ),
                                                    ),
                                                },
                                            ),
                                            (
                                                "city".to_string(),
                                                Value {
                                                    kind: Some(
                                                        tucana::shared::value::Kind::StringValue(
                                                            "Anytown".to_string(),
                                                        ),
                                                    ),
                                                },
                                            ),
                                            (
                                                "zipcode".to_string(),
                                                Value {
                                                    kind: Some(
                                                        tucana::shared::value::Kind::NumberValue(
                                                            12345.0,
                                                        ),
                                                    ),
                                                },
                                            ),
                                        ]),
                                    },
                                )),
                            },
                        ),
                    ]),
                },
            )),
        };

        // Test basic top-level fields
        assert_eq!(
            expect_kind("name", &value),
            Some(tucana::shared::value::Kind::StringValue("John".to_string()))
        );

        // Test nested fields
        assert_eq!(
            expect_kind("address.street", &value),
            Some(tucana::shared::value::Kind::StringValue(
                "123 Main St".to_string()
            ))
        );
        assert_eq!(
            expect_kind("address.city", &value),
            Some(tucana::shared::value::Kind::StringValue(
                "Anytown".to_string()
            ))
        );
        assert_eq!(
            expect_kind("address.zipcode", &value),
            Some(tucana::shared::value::Kind::NumberValue(12345.0))
        );

        // Test nonexistent fields
        assert_eq!(expect_kind("address.country", &value), None);
        assert_eq!(expect_kind("phone", &value), None);
        assert_eq!(expect_kind("address.street.number", &value), None);

        assert!(exists_path("address.city", &value));
        assert!(!exists_path("address.street.number", &value));
    }
}
