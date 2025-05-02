use super::violation::{
    DataTypeRuleError, DataTypeRuleViolation, InvalidFormatRuleViolation,
    ItemOfCollectionRuleViolation,
};
use tucana::shared::{
    value::Kind, DataType, DataTypeContainsTypeRuleConfig, DataTypeItemOfCollectionRuleConfig,
    Value,
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
    data_types: &Vec<DataType>,
    body: &Value,
) -> Result<(), DataTypeRuleError> {
    let real_body = match &body.kind {
        Some(body) => body.clone(),
        None => {
            return Err(DataTypeRuleError {
                violations: vec![DataTypeRuleViolation::InvalidFormat(
                    InvalidFormatRuleViolation {
                        expected_format: rule.data_type_identifier,
                        value: String::from("other"),
                    },
                )],
            });
        }
    };

    match real_body {
        Kind::ListValue(list) => {
            let real_data_type = data_types
                .iter()
                .find(|data_type| data_type.identifier == rule.data_type_identifier)
                .cloned();

            if let Some(data_type) = real_data_type {
                if list.values.contains(data_type) {
                    return Ok(());
                } else {
                    
                }
            }
        }
        _ => {
            return Err(DataTypeRuleError {
                violations: vec![DataTypeRuleViolation::InvalidFormat(
                    InvalidFormatRuleViolation {
                        expected_format: rule.data_type_identifier,
                        value: String::from("other"),
                    },
                )],
            });
        }
    }

    Ok(())
}
