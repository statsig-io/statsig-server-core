use std::borrow::Cow;
use std::sync::{Arc, RwLock};

use uaparser::{Parser, UserAgentParser as ExtUserAgentParser};

use crate::{
    log_d, log_e, statsig_user_internal::StatsigUserInternal, unwrap_or_return_with, DynamicValue,
};

use super::dynamic_string::DynamicString;

lazy_static::lazy_static! {
    static ref PARSER: Arc<RwLock<Option<ExtUserAgentParser>>> = Arc::from(RwLock::from(None));
}

const TAG: &str = "UserAgentParser";

pub struct UserAgentParser;

impl UserAgentParser {
    pub fn get_value_from_user_agent(
        user: &StatsigUserInternal,
        field: &Option<DynamicString>,
    ) -> Option<DynamicValue> {
        let field_lowered = match field {
            Some(f) => f.lowercased_value.as_str(),
            _ => return None,
        };

        let user_agent = match &user.user_data.user_agent {
            Some(ua) => ua.string_value.as_ref(),
            _ => return None,
        }?;

        if user_agent.len() > 1000 {
            return None;
        }

        let lock = unwrap_or_return_with!(PARSER.read().ok(), || {
            log_e!(TAG, "Failed to acquire read lock on parser");
            None
        });

        let parser = unwrap_or_return_with!(lock.as_ref(), || {
            log_e!(TAG, "Attempted to use parser after it was loaded");
            None
        });

        fn get_json_version(
            major: Option<Cow<str>>,
            minor: Option<Cow<str>>,
            patch: Option<Cow<str>>,
        ) -> DynamicValue {
            let fallback = Cow::Borrowed("0");
            DynamicValue::from(format!(
                "{}.{}.{}",
                major.unwrap_or(fallback.clone()),
                minor.unwrap_or(fallback.clone()),
                patch.unwrap_or(fallback.clone())
            ))
        }

        let parsed = parser.parse(user_agent);
        match field_lowered {
            "os_name" | "osname" => Some(DynamicValue::from(parsed.os.family.to_string())),
            "os_version" | "osversion" => {
                let os = parsed.os;
                Some(get_json_version(os.major, os.minor, os.patch))
            }
            "browser_name" | "browsername" => {
                Some(DynamicValue::from(parsed.user_agent.family.to_string()))
            }
            "browser_version" | "browserversion" => {
                let ua = parsed.user_agent;
                Some(get_json_version(ua.major, ua.minor, ua.patch))
            }
            _ => None,
        }
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

        let bytes = include_bytes!("../../resources/ua_parser_regex.yaml");
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
