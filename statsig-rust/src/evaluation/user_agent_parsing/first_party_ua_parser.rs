use crate::{dyn_value, log_w, DynamicValue};

use super::statsig_uaparser::UaParser;

const TAG: &str = "FirstPartyUserAgentParser";

pub struct FirstPartyUserAgentParser;

impl FirstPartyUserAgentParser {
    pub fn get_value_from_user_agent(field: &str, user_agent: &str) -> Option<DynamicValue> {
        match field {
            "os_name" | "osname" => {
                let os = UaParser::parse_os(user_agent);
                Some(dyn_value!(os.name))
            }
            "os_version" | "osversion" => {
                let os = UaParser::parse_os(user_agent);
                Some(dyn_value!(os
                    .version
                    .get_version_string()
                    .unwrap_or("0.0.0".to_string())))
            }
            "browser_name" | "browsername" => {
                let browser = UaParser::parse_browser(user_agent);
                Some(dyn_value!(browser.name))
            }
            "browser_version" | "browserversion" => {
                let browser = UaParser::parse_browser(user_agent);
                Some(dyn_value!(browser
                    .version
                    .get_version_string()
                    .unwrap_or("0.0.0".to_string())))
            }
            _ => {
                log_w!(TAG, "Unsupported field: {}", field);
                None
            }
        }
    }
}
