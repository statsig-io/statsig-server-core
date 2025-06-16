use std::borrow::Cow;
use std::sync::{Arc, RwLock};

use uaparser::{Parser, UserAgentParser as ExtUserAgentParser};

use crate::{dyn_value, log_w, DynamicValue};
use crate::{log_d, log_e, unwrap_or_return_with, user::StatsigUserInternal};

use super::dynamic_string::DynamicString;

lazy_static::lazy_static! {
    static ref PARSER: Arc<RwLock<Option<ExtUserAgentParser>>> = Arc::from(RwLock::from(None));
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

        let lock = unwrap_or_return_with!(PARSER.read().ok(), || {
            *override_reason = Some(UNINITIALIZED_REASON);
            log_e!(TAG, "Failed to acquire read lock on parser");
            None
        });

        let parser = unwrap_or_return_with!(lock.as_ref(), || {
            *override_reason = Some(UNINITIALIZED_REASON);
            log_w!(TAG, "Failed to load UA Parser. Did you disable UA Parser or did not wait for user agent to init. Check StatsigOptions configuration");
            None
        });

        fn get_json_version(
            major: Option<Cow<str>>,
            minor: Option<Cow<str>>,
            patch: Option<Cow<str>>,
        ) -> String {
            let mut result = String::new();
            result += &major.unwrap_or(Cow::Borrowed("0"));
            result += ".";
            result += &minor.unwrap_or(Cow::Borrowed("0"));
            result += ".";
            result += &patch.unwrap_or(Cow::Borrowed("0"));
            result
        }

        let result = match field_lowered {
            "os_name" | "osname" => {
                let os = parser.parse_os(user_agent);
                os.family.to_string()
            }
            "os_version" | "osversion" => {
                let os = parser.parse_os(user_agent);
                get_json_version(os.major, os.minor, os.patch)
            }
            "browser_name" | "browsername" => {
                let user_agent = parser.parse_user_agent(user_agent);
                user_agent.family.to_string()
            }
            "browser_version" | "browserversion" => {
                let user_agent = parser.parse_user_agent(user_agent);
                get_json_version(user_agent.major, user_agent.minor, user_agent.patch)
            }
            _ => return None,
        };

        // Some(EvaluatorValue::from(result))
        Some(dyn_value!(result))
    }

    pub fn load_parser() {
        match PARSER.read() {
            Ok(lock) => {
                if lock.is_some() {
                    log_d!(TAG, "Parser already loaded");
                    return;
                }
            }
            Err(e) => {
                log_e!(TAG, "Failed to acquire read lock on parser: {}", e);
                return;
            }
        }

        log_d!(TAG, "Loading User Agent Parser...");

        let bytes = include_bytes!("../../resources/ua_parser_regex_lite.yaml");
        let parser = match ExtUserAgentParser::from_bytes(bytes) {
            Ok(parser) => parser,
            Err(e) => {
                log_e!(TAG, "Failed to load parser: {}", e);
                return;
            }
        };

        match PARSER.write() {
            Ok(mut lock) => {
                *lock = Some(parser);
                log_d!(TAG, "User Agent Parser Successfully Loaded");
            }
            Err(e) => {
                log_e!(TAG, "Failed to acquire write lock on parser: {}", e);
            }
        }
    }
}
