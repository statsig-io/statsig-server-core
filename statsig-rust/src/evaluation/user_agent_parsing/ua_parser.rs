use crate::evaluation::dynamic_string::DynamicString;
use crate::user::StatsigUserInternal;
use crate::{log_w, DynamicValue};

use super::experimental_ua_parser::ExperimentalUserAgentParser;
use super::third_party_ua_parser::ThirdPartyUserAgentParser;

lazy_static::lazy_static! {
    static ref USER_AGENT_STRING: Option<DynamicString> = Some(DynamicString::from("userAgent".to_string()));
}

const TAG: &str = "UserAgentParser";
const UNINITIALIZED_REASON: &str = "UAParserNotLoaded";

pub struct UserAgentParser;

impl UserAgentParser {
    pub fn get_value_from_user_agent(
        user: &StatsigUserInternal,
        field: &Option<DynamicString>,
        override_reason: &mut Option<&str>,
        use_experimental_ua_parser: bool,
    ) -> Option<DynamicValue> {
        let field_lowered = match field {
            Some(f) => f.lowercased_value.as_str(),
            _ => return None,
        };

        let user_agent = match user.get_user_value(&USER_AGENT_STRING) {
            Some(v) => match &v.string_value {
                Some(s) => &s.value,
                _ => return None,
            },
            None => return None,
        };

        if user_agent.len() > 1000 {
            return None;
        }

        if use_experimental_ua_parser {
            ExperimentalUserAgentParser::get_value_from_user_agent(field_lowered, user_agent)
        } else {
            let result =
                ThirdPartyUserAgentParser::get_value_from_user_agent(field_lowered, user_agent);

            match result {
                Ok(v) => v,
                Err(_) => {
                    *override_reason = Some(UNINITIALIZED_REASON);
                    log_w!(TAG, "Failed to load UA Parser. Check StatsigOptions.disable_user_agent_parsing and or wait_for_user_agent_init");
                    None
                }
            }
        }
    }

    pub fn load_parser() {
        ThirdPartyUserAgentParser::load_parser();
    }
}
