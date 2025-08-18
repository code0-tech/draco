use tucana::shared::{FlowSetting, value::Kind};

pub mod config;
pub mod runner;
pub mod store;
pub mod traits;

pub fn extract_flow_setting_field(
    settings: &Vec<FlowSetting>,
    def_key: &str,
    field_name: &str,
) -> Option<String> {
    settings.iter().find_map(|setting| {
        if setting.flow_setting_id != def_key {
            return None;
        }

        let obj = setting.object.as_ref()?;
        obj.fields.iter().find_map(|(k, v)| {
            if k == field_name {
                if let Some(Kind::StringValue(s)) = &v.kind {
                    return Some(s.clone());
                }
            }
            None
        })
    })
}
