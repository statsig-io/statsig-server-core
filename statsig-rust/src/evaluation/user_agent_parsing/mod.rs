pub mod experimental_ua_parser;
pub mod statsig_uaparser;
pub mod third_party_ua_parser;
pub mod ua_parser;

use serde::Serialize;
pub use ua_parser::UserAgentParser;

#[derive(Serialize, Debug)]
pub struct ParsedUserAgentValue {
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub browser_name: Option<String>,
    pub browser_version: Option<String>,
}
