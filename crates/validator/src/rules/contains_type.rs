use crate::{get_data_type_by_id, verify_data_type_rules};

use super::violation::{
    DataTypeIdentifierNotPresentRuleViolation, DataTypeRuleError, DataTypeRuleViolation,
    GenericKeyNotAllowedRuleViolation, InvalidFormatRuleViolation,
};
use tucana::shared::{
    data_type_identifier::Type, value::Kind, DataType, DataTypeContainsTypeRuleConfig, Value,
};

/// # Item of Collection Validation
///
/// This function validates if a value is contained within a predefined collection of allowed items.
///
/// ## Process:
/// 1. Checks if the provided value is present in the collection of allowed items
///
/// ## Error Handling:
/// - Returns an `ItemOfCollectionRuleViolation` if the value is not found in the collection
///
pub fn apply_contains_type(
    rule: DataTypeContainsTypeRuleConfig,
    available_data_types: &Vec<DataType>,
    body: &Value,
) -> Result<(), DataTypeRuleError> {
    let identifier = match rule.data_type_identifier {
        Some(optional_data_type) => {
            if let Some(data_type) = optional_data_type.r#type {
                match data_type {
                    Type::DataTypeIdentifier(id) => id,
                    _ => {
                        return Err(DataTypeRuleError {
                            violations: vec![DataTypeRuleViolation::GenericKeyNotAllowed(
                                GenericKeyNotAllowedRuleViolation {
                                    key: "identifier".to_string(),
                                },
                            )],
                        })
                    }
                }
            } else {
                return Err(DataTypeRuleError {
                    violations: vec![DataTypeRuleViolation::DataTypeIdentifierNotPresent(
                        DataTypeIdentifierNotPresentRuleViolation {
                            identifier: "identifier".to_string(),
                        },
                    )],
                });
            }
        }
        None => {
            return Err(DataTypeRuleError {
                violations: vec![DataTypeRuleViolation::DataTypeIdentifierNotPresent(
                    DataTypeIdentifierNotPresentRuleViolation {
                        identifier: "identifier".to_string(),
                    },
                )],
            });
        }
    };

    let real_body = match &body.kind {
        Some(body) => body.clone(),
        None => {
            return Err(DataTypeRuleError {
                violations: vec![DataTypeRuleViolation::InvalidFormat(
                    InvalidFormatRuleViolation {
                        expected_format: identifier,
                        value: String::from("other"),
                    },
                )],
            });
        }
    };

    match real_body {
        Kind::ListValue(list) => {
            let real_data_type = get_data_type_by_id(available_data_types, &identifier);

            if let Some(data_type) = real_data_type {
                let mut rule_errors: Option<DataTypeRuleError> = None;

                for value in list.values {
                    match verify_data_type_rules(value, data_type.clone(), &available_data_types) {
                        Ok(_) => {}
                        Err(errors) => {
                            rule_errors = Some(errors);
                        }
                    }
                }

                if let Some(errors) = rule_errors {
                    return Err(errors);
                } else {
                    return Ok(());
                }
            }
        }
        _ => {
            return Err(DataTypeRuleError {
                violations: vec![DataTypeRuleViolation::InvalidFormat(
                    InvalidFormatRuleViolation {
                        expected_format: identifier,
                        value: String::from("other"),
                    },
                )],
            });
        }
    }

    Ok(())
}
