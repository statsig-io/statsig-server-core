pub mod experimental_ua_parser;
pub mod statsig_uaparser;
pub mod third_party_ua_parser;
pub mod ua_parser;

use serde::Serialize;
pub use ua_parser::UserAgentParser;

use crate::interned_string::InternedString;

#[derive(Serialize, Debug)]
pub struct ParsedUserAgentValue {
    pub os_name: Option<InternedString>,
    pub os_version: Option<InternedString>,
    pub browser_name: Option<InternedString>,
    pub browser_version: Option<InternedString>,
}
