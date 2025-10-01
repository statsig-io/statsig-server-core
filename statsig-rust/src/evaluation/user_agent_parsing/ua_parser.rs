use std::sync::Arc;

use crate::evaluation::dynamic_string::DynamicString;
use crate::evaluation::user_agent_parsing::ParsedUserAgentValue;
use crate::interned_string::InternedString;
use crate::user::StatsigUserInternal;
use crate::{log_w, unwrap_or_return, DynamicValue, StatsigOptions, StatsigUser};

use super::first_party_ua_parser::FirstPartyUserAgentParser;
use super::third_party_ua_parser::ThirdPartyUserAgentParser;

lazy_static::lazy_static! {
    static ref USER_AGENT_STRING: Option<DynamicString> = Some(DynamicString::from("userAgent".to_string()));
}

const TAG: &str = "UserAgentParser";
const UNINITIALIZED_REASON: &str = "UAParserNotLoaded";

pub struct UserAgentParser;

fn get_experimental_ua_value(key: &str, ua: &str) -> Option<InternedString> {
    FirstPartyUserAgentParser::get_value_from_user_agent(key, ua)
        .and_then(|v| v.string_value.map(|s| s.value))
}

fn get_third_party_ua_value(key: &str, ua: &str) -> Option<InternedString> {
    ThirdPartyUserAgentParser::get_value_from_user_agent(key, ua)
        .ok()
        .flatten()
        .and_then(|dv| dv.string_value.map(|s| s.value))
}

impl UserAgentParser {
    pub fn get_value_from_user_agent(
        user: &StatsigUserInternal,
        field: &Option<DynamicString>,
        override_reason: &mut Option<&str>,
        use_third_party_ua_parser: bool,
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

        if use_third_party_ua_parser {
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
        } else {
            FirstPartyUserAgentParser::get_value_from_user_agent(field_lowered, user_agent)
        }
    }

    pub fn load_parser() {
        ThirdPartyUserAgentParser::load_parser();
    }

    pub fn get_parsed_user_agent_value_for_user(
        user: &StatsigUser,
        options: &Arc<StatsigOptions>,
    ) -> Option<ParsedUserAgentValue> {
        let user_agent_str = unwrap_or_return!(user.get_user_agent(), None);
        match options.use_third_party_ua_parser {
            Some(false) => Some(ParsedUserAgentValue {
                os_name: get_third_party_ua_value("os_name", user_agent_str),
                os_version: get_third_party_ua_value("os_version", user_agent_str),
                browser_name: get_third_party_ua_value("browser_name", user_agent_str),
                browser_version: get_third_party_ua_value("browser_version", user_agent_str),
            }),
            _ => Some(ParsedUserAgentValue {
                os_name: get_experimental_ua_value("os_name", user_agent_str),
                os_version: get_experimental_ua_value("os_version", user_agent_str),
                browser_name: get_experimental_ua_value("browser_name", user_agent_str),
                browser_version: get_experimental_ua_value("browser_version", user_agent_str),
            }),
        }
    }
}
