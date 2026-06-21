use tucana::shared::{ValidationFlow, Value, value::Kind};

use super::types::{AuthenticationType, is_unauthenticated_value};

pub(super) enum FlowAuthConfig {
    Unauthenticated,
    Authenticated(AuthenticationType),
    Invalid,
}

pub(super) fn flow_auth_config(flow: &ValidationFlow) -> FlowAuthConfig {
    let Some(raw_auth_type) = flow_setting_as_string(flow, "httpAuth") else {
        return FlowAuthConfig::Unauthenticated;
    };

    if is_unauthenticated_value(raw_auth_type) {
        return FlowAuthConfig::Unauthenticated;
    }

    match AuthenticationType::parse(raw_auth_type) {
        Some(auth_type) => FlowAuthConfig::Authenticated(auth_type),
        None => {
            log::warn!(
                "auth config invalid: flow_id={} httpAuth={:?}",
                flow.flow_id,
                raw_auth_type
            );
            FlowAuthConfig::Invalid
        }
    }
}

pub(super) fn flow_setting_value<'a>(
    flow: &'a ValidationFlow,
    flow_setting_id: &str,
) -> Option<&'a Value> {
    flow.settings
        .iter()
        .find(|setting| setting.flow_setting_id == flow_setting_id)
        .and_then(|setting| setting.value.as_ref())
}

fn flow_setting_as_string<'a>(flow: &'a ValidationFlow, flow_setting_id: &str) -> Option<&'a str> {
    flow_setting_value(flow, flow_setting_id)
        .and_then(|value| value.kind.as_ref())
        .and_then(|kind| match kind {
            Kind::StringValue(value) => Some(value.as_str()),
            // Missing or null/non-string httpAuth means the flow remains public.
            _ => None,
        })
}
