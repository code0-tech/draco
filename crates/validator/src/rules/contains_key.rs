use super::violation::ContainsKeyRuleViolation;
use super::violation::DataTypeRuleError;
use super::violation::DataTypeRuleViolation;
use super::violation::MissingDataTypeRuleDefinition;
use crate::path::path::expect_kind;
use crate::{verify_body, ContainsRule};
use tucana::shared::value::Kind;
use tucana::shared::DataType;
use tucana::shared::DataTypeContainsKeyRuleConfig;
use tucana::shared::Flow;
use tucana::shared::Value;

/// # Data Type Validation Behavior
///
/// This function checks if a specific key exists in the JSON body and validates
/// if its value matches the expected data type.
///
/// ## Process:
/// 1. Searches for the specified key in the JSON body
/// 2. If the key is found, retrieves the associated data type definition from the flow
/// 3. Validates that the value matches the expected data type
///
/// ## Error Handling:
/// - Returns a `ContainsKeyRuleViolation` if the specified key is not found in the body
/// - Returns a `MissingDataTypeRuleDefinition` if the referenced data type doesn't exist
/// - Returns validation errors if the value doesn't match the expected data type
pub fn apply_contains_key(
    rule: DataTypeContainsKeyRuleConfig,
    body: Value,
    flow: Flow,
) -> Result<(), DataTypeRuleError> {
    println!("{:?} on body {:?}", rule, body);
    panic!("TODO!");
    /*
    if let Some(Kind::StructValue(_)) = &body.kind {
        let value = match expect_kind(&rule.key, &body) {
            Some(value) => Value {
                kind: Some(value.to_owned()),
            },
            None => {
                let error = ContainsKeyRuleViolation {
                    missing_key: rule.key,
                };

                return Err(DataTypeRuleError {
                    violations: vec![DataTypeRuleViolation::ContainsKey(error)],
                });
            }
        };

        let data_type = match get_data_type_by_id(&flow, rule.r#type.clone()) {
            Some(data_type) => data_type,
            None => {
                let error = MissingDataTypeRuleDefinition {
                    missing_type: rule.r#type,
                };

                return Err(DataTypeRuleError {
                    violations: vec![DataTypeRuleViolation::MissingDataType(error)],
                });
            }
        };

        match verify_body(flow, value, data_type) {
            Ok(()) => Ok(()),
            Err(e) => Err(e),
        }
    } else {
        return Err(DataTypeRuleError {
            violations: vec![DataTypeRuleViolation::ContainsKey(
                ContainsKeyRuleViolation {
                    missing_key: rule.key.clone(),
                },
            )],
        });
    }
     */
}

fn get_data_type_by_id(flow: &Flow, str_id: String) -> Option<DataType> {
    let id = str_id.parse::<i32>().unwrap_or(1211);

    flow.data_types
        .iter()
        .find(|data_type| data_type.variant == id)
        .cloned()
}
