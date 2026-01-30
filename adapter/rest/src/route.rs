use base::traits::IdentifiableFlow;
use tucana::shared::{ValidationFlow, value::Kind};

pub struct RequestRoute {
    pub url: String,
    pub method: hyper::Method,
}

// Checks if the Method and Url matches any of the
// Flows that matched the original slug pattern for project
// Only if both matched, it will return true
impl IdentifiableFlow for RequestRoute {
    fn identify(&self, flow: &ValidationFlow) -> bool {
        // Get Method of the FlowSetting
        let method_str = flow
            .settings
            .iter()
            .find(|s| s.flow_setting_id == "HTTP_METHOD")
            .and_then(|s| s.value.as_ref())
            .and_then(|v| v.kind.as_ref())
            .and_then(|k| match k {
                Kind::StringValue(s) => Some(s.as_str()),
                _ => None,
            });

        log::debug!(
            "Comparing flows method: {:?} with request route: {}",
            method_str,
            self.method.as_str()
        );

        if let Some(method) = method_str {
            if method != self.method.as_str() {
                log::debug!("Method didn't eq");
                return false;
            }
        } else {
            log::debug!("Method didn't eq");
            return false;
        }
        // Get URL of the FlowSetting
        let regex_str_v = flow
            .settings
            .iter()
            .find(|s| s.flow_setting_id == "HTTP_URL");

        log::debug!("Extracted: {:?} as HTTP_URL", &regex_str_v);

        let regex_str = regex_str_v
            .and_then(|s| s.value.as_ref())
            .and_then(|v| v.kind.as_ref())
            .and_then(|k| match k {
                Kind::StringValue(s) => Some(s.as_str()),
                _ => None,
            });

        let Some(regex_str) = regex_str else {
            log::debug!("Regex was empty");
            return false;
        };

        log::debug!(
            "Comparing regex {} with literal route: {}",
            regex_str,
            self.url
        );

        // Check if the request is matching
        match regex::Regex::new(regex_str) {
            Ok(regex) => {
                log::debug!("found a match for {}", regex_str);
                regex.is_match(&self.url)
            }
            Err(err) => {
                log::error!("Failed to compile regex: {}", err);
                false
            }
        }
    }
}

pub fn extract_slug_from_path(path: &str) -> Option<&str> {
    let trimmed = path.trim_start_matches('/');
    trimmed.split('/').next().filter(|s| !s.is_empty())
}
