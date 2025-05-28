use super::violation::ContainsKeyRuleViolation;
use super::violation::DataTypeRuleError;
use super::violation::DataTypeRuleViolation;
use super::violation::MissingDataTypeRuleDefinition;
use crate::get_data_type_by_id;
use crate::verify_data_type_rules;
use tucana::shared::helper::path::expect_kind;
use tucana::shared::value::Kind;
use tucana::shared::DataType;
use tucana::shared::DataTypeContainsKeyRuleConfig;
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
    body: &Value,
    available_data_types: &Vec<DataType>,
) -> Result<(), DataTypeRuleError> {
    todo!("Adjsut to Generic Keys");
    /*
    if let Some(Kind::StructValue(_)) = &body.kind {
        let value = match expect_kind(&rule.data_type_identifier, &body) {
            Some(value) => Value {
                kind: Some(value.to_owned()),
            },
            None => {
                let error = ContainsKeyRuleViolation {
                    missing_key: rule.data_type_identifier,
                };

                return Err(DataTypeRuleError {
                    violations: vec![DataTypeRuleViolation::ContainsKey(error)],
                });
            }
        };

        let data_type = match get_data_type_by_id(&available_data_types, &rule.data_type_identifier)
        {
            Some(data_type) => data_type,
            None => {
                let error = MissingDataTypeRuleDefinition {
                    missing_type: rule.data_type_identifier,
                };

                return Err(DataTypeRuleError {
                    violations: vec![DataTypeRuleViolation::MissingDataType(error)],
                });
            }
        };

        return verify_data_type_rules(value, data_type, available_data_types);
    } else {
        return Err(DataTypeRuleError {
            violations: vec![DataTypeRuleViolation::ContainsKey(
                ContainsKeyRuleViolation {
                    missing_key: rule.data_type_identifier.clone(),
                },
            )],
        });
    }
     */
}
