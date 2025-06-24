use crate::{dyn_value, log_d, log_e, DynamicValue};
use std::borrow::Cow;
use std::sync::{Arc, RwLock};
use uaparser::{Parser, UserAgentParser as ExtUserAgentParser};

lazy_static::lazy_static! {
    static ref PARSER: Arc<RwLock<Option<ExtUserAgentParser>>> = Arc::from(RwLock::from(None));
}

const TAG: &str = "ThirdPartyUserAgentParser";

pub struct ThirdPartyUserAgentParser;

impl ThirdPartyUserAgentParser {
    pub fn get_value_from_user_agent(
        field: &str,
        user_agent: &str,
    ) -> Result<Option<DynamicValue>, &'static str> {
        let lock = PARSER.read().map_err(|_| "lock_failure")?;
        let parser = lock.as_ref().ok_or("parser_not_loaded")?;

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

        let result = match field {
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
            _ => return Ok(None),
        };

        Ok(Some(dyn_value!(result)))
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

        let bytes = include_bytes!("../../../resources/ua_parser_regex_lite.yaml");
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
