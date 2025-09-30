use super::tokenizer::{Token, Tokenizer};

pub struct UaParser;

impl UaParser {
    pub fn parse_os(agent: &str) -> ParserResult<'_> {
        let result = Tokenizer::run(agent);

        if let Some(token) = &result.possible_os_token {
            return create_res(token.tag, token.get_version());
        }

        for token in &result.tokens {
            if token.tag == "ATV OS X" {
                return create_res("ATV OS X", None);
            }

            if token.tag == "iPhone OS" || token.tag == "iOS" {
                return create_res("iOS", token.get_version());
            }

            if token.tag == "CPU OS" && result.ios_hint {
                return create_res("iOS", token.get_version());
            }

            if token.tag == "Version" && result.ios_hint && result.macos_hint {
                return create_res("iOS", token.get_version());
            }

            if token.tag == "CFNetwork" {
                return create_res("iOS", None);
            }

            if token.tag == "Android" {
                return create_res(token.tag, token.get_version());
            }

            if token.tag.starts_with("Android") {
                return create_res("Android", None);
            }

            if token.tag == "Chromecast" {
                return create_res("Chromecast", None);
            }

            if token.tag == "Red Hat" {
                return create_res("Red Hat", None);
            }

            if token.tag == "Kindle" {
                return create_res("Kindle", token.get_version());
            }

            if token.tag == "Ubuntu" {
                return create_res("Ubuntu", token.get_version());
            }
        }

        if result.ios_hint {
            return create_res("iOS", None);
        }

        if result.macos_hint {
            return create_res("Mac OS X", None);
        }

        if result.windows_hint {
            return create_res("Windows", None);
        }

        if result.linux_hint {
            return create_res("Linux", None);
        }

        create_res("Other", None)
    }

    pub fn parse_browser(agent: &str) -> ParserResult<'_> {
        let result = Tokenizer::run(agent);

        if let Some(token) = &result.possible_browser_token {
            return create_res(token.tag, token.get_version());
        }

        let mut android_token: Option<&Token> = None;
        let mut chrome_token: Option<&Token> = None;
        let mut version_token: Option<&Token> = None;

        for token in &result.tokens {
            if token.tag == "Firefox" {
                if result.mobile_hint {
                    return create_res("Firefox Mobile", token.get_version());
                }

                return create_res("Firefox", token.get_version());
            }

            if token.tag == "Android" {
                android_token = Some(token);
                continue;
            }

            if token.tag == "Version" {
                version_token = Some(token);
                continue;
            }

            if token.tag == "Yahoo! Slurp" {
                return create_res("Yahoo! Slurp", None);
            }

            if token.tag == "Silk" {
                if result.playstation_hint {
                    return create_res("NetFront NX", None);
                }

                return create_res("Amazon Silk", token.get_version());
            }

            if token.tag == "NetFront NX" {
                return create_res("NetFront NX", token.get_version());
            }

            if token.tag == "YaBrowser" {
                return create_res("Yandex Browser", token.get_version());
            }

            if token.tag == "Edge" && result.mobile_hint {
                return create_res("Edge Mobile", token.get_version());
            }

            if token.tag == "Edge" {
                return create_res("Edge", token.get_version());
            }

            if token.tag == "Opera" {
                if result.mobile_hint {
                    return create_res("Opera Mobile", token.get_version());
                }
                return create_res("Opera", token.get_version());
            }

            if token.tag == "Chrome" {
                chrome_token = Some(token);
                continue;
            }

            if token.tag == "axios" {
                return create_res("axios", token.get_version());
            }

            if token.tag == "HeadlessChrome" {
                return create_res("HeadlessChrome", token.get_version());
            }
        }

        if let Some(token) = chrome_token {
            if version_token.is_some() {
                return create_res("Chrome Mobile WebView", token.get_version());
            }

            if result.mobile_hint && token.version.is_some() && !result.huawei_hint {
                return create_res("Chrome Mobile", token.get_version());
            }

            if token.version.is_none() {
                if let Some(token) = android_token {
                    return create_res("Android", token.get_version());
                }
            }

            return create_res("Chrome", token.get_version());
        }

        if let Some(token) = android_token {
            return create_res("Android", token.get_version());
        }

        if result.cfnetwork_hint {
            if result.tokens[0].tag == "NetworkingExtension" {
                return create_res("CFNetwork", result.tokens[1].get_version());
            }
            return create_res(result.tokens[0].tag, result.tokens[0].get_version());
        }

        if result.safari_hint {
            let version = version_token.and_then(|t| t.get_version());

            if result.mobile_hint && !result.macos_hint {
                // UA string has this “Mobile” flag likely for compatibility or simulation purposes but it's running ins macos
                return create_res("Mobile Safari", version);
            }

            return create_res("Safari", version);
        }

        if result.ios_hint {
            return create_res(
                "Mobile Safari UI/WKWebView",
                result.possible_os_token.and_then(|o| o.get_version()),
            );
        }

        if result.crawler_hint {
            return create_res("crawler", None);
        }
        create_res("Other", None)
    }
}

#[derive(Debug, Default)]
pub struct Version<'a> {
    pub major: Option<&'a str>,
    pub minor: Option<&'a str>,
    pub patch: Option<&'a str>,
    pub patch_minor: Option<&'a str>,
}

impl<'a> Version<'a> {
    pub fn major(major: &'a str) -> Self {
        Self {
            major: Some(major),
            minor: None,
            patch: None,
            patch_minor: None,
        }
    }

    pub fn get_version_string(&self) -> Option<String> {
        let major = self.major?;

        let mut version = String::new();

        version.push_str(major);

        if let Some(minor) = self.minor {
            version.push('.');
            version.push_str(minor);
        }

        if let Some(patch) = self.patch {
            version.push('.');
            version.push_str(patch);
        }

        if let Some(patch_minor) = self.patch_minor {
            version.push('.');
            version.push_str(patch_minor);
        }

        Some(version)
    }
}

pub struct ParserResult<'a> {
    pub name: &'a str,
    pub version: Version<'a>,
}

fn create_res<'a>(name: &'a str, version: Option<Version<'a>>) -> ParserResult<'a> {
    ParserResult {
        name,
        version: version.unwrap_or_default(),
    }
}
