use crate::unwrap_or_return;

use super::{window_iter::WindowIter, Version};

pub struct Tokenizer;

impl Tokenizer {
    // Ideal UserAgent format: <product>/<product-version> (<os-information>) <engine> (<platform-details>) <optional-details>
    pub fn run(input: &str) -> TokenizerResult<'_> {
        let mut result = TokenizerResult::default();
        let mut win: WindowIter<'_> = WindowIter::new(input);
        while !win.is_empty() {
            let (curr, next1, next2, next3) = win.get_window();
            let (curr, next1, next2, next3) = (
                trim_invalid_chars(curr),
                trim_invalid_chars(next1),
                trim_invalid_chars(next2),
                trim_invalid_chars(next3),
            );

            let curr = match curr {
                Some(val) => val,
                None => {
                    win.slide_window_by(1);
                    continue;
                }
            };

            if curr.starts_with("AppleTV") {
                result.add_tag("ATV OS X", None);
                win.slide_window_by(1);
            } else if curr == "like" && next1 == Some("Mac") && next2 == Some("OS") {
                win.slide_window_by(3);
            }
            // Mac OS X
            else if curr == "Mac" && next1 == Some("OS") && next2 == Some("X") {
                result.macos_hint = true;

                result.add_possible_os_tag("Mac OS X", consume_if_numeric(&mut win, next3));

                win.slide_window_by(2);
            }
            // iPhone OS
            else if curr == "iOS" {
                result.add_possible_os_tag("iOS", consume_if_numeric(&mut win, next1));
                result.ios_hint = true;
                win.slide_window_by(1);
            } else if curr == "iPhone" && next1 == Some("OS") {
                result.add_possible_os_tag("iOS", consume_if_numeric(&mut win, next2));

                win.slide_window_by(1);
            }
            // iPad
            else if curr.starts_with("iPad") {
                result.ios_hint = true;

                let mut parts = curr.split("iPad");
                let _ = parts.next();
                result.add_tag("iPad", trim_invalid_chars(parts.next()));
            }
            // iPhone
            else if curr.starts_with("iPhone") {
                result.ios_hint = true;

                let mut parts = curr.split("iPhone");
                let _ = parts.next();
                result.add_tag("iPhone", trim_invalid_chars(parts.next()));
            }
            // iPhone (Apple)
            else if curr.starts_with("Apple-iPhone7C2") {
                result.ios_hint = true;

                result.add_tag("iPhone", None);
            }
            // CPU OS
            else if curr == "CPU" && next1 == Some("OS") {
                result.add_tag("CPU OS", consume_if_numeric(&mut win, next2));
                win.slide_window_by(1);
            }
            // Chrome OS
            else if curr == "CrOS" {
                let mut version = consume_if_numeric(&mut win, next1);
                if version.is_none() {
                    win.slide_window_by(1);
                    version = consume_if_numeric(&mut win, next2);
                }

                result.add_possible_os_tag("Chrome OS", version);
            }
            // Chromecast
            else if curr == "CrKey" {
                result.add_possible_os_tag("Chromecast", None);
            }
            // PlayStation
            else if curr == "PlayStation" {
                result.playstation_hint = true;

                result.add_tag("PlayStation", None);
            }
            // Android
            else if curr == "Android" {
                result.add_possible_os_tag("Android", consume_if_numeric(&mut win, next1));
            }
            // Windows Phone
            else if curr == "Windows" && next1 == Some("Phone") {
                result.add_possible_os_tag("Windows Phone", consume_if_numeric(&mut win, next2));
                win.slide_window_by(1);
            }
            // Windows
            else if curr.starts_with("Windows") {
                result.windows_hint = true;

                let version = if next1 == Some("NT") {
                    consume_if_numeric(&mut win, next2).inspect(|_| {
                        win.slide_window_by(1); // extra slide to skip the NT
                    })
                } else if next1.is_some_and(|s| s.starts_with("XP")) {
                    win.slide_window_by(1);
                    Some("XP")
                } else {
                    consume_if_numeric(&mut win, next1)
                };

                result.add_possible_os_tag("Windows", version);
            }
            // Yahoo Slurp
            else if curr == "Yahoo!" && next1 == Some("Slurp") {
                result.add_tag("Yahoo! Slurp", None);
                win.slide_window_by(1);
            }
            // Red Hat
            else if curr == "Red" && next1 == Some("Hat") {
                result.add_possible_os_tag("Red Hat", None);

                win.slide_window_by(1);
            }
            // Ubuntu
            else if curr == "Ubuntu" {
                result.add_possible_os_tag("Ubuntu", consume_if_numeric(&mut win, next1));
            }
            // Mobile
            else if curr == "Mobile" {
                result.mobile_hint = true;

                result.add_tag("Mobile", None);
            }
            // Linux
            else if curr == "Linux" || curr == "linux" {
                result.linux_hint = true;
                result.add_tag("Linux", None);
            }
            // Nintendo
            else if curr == "Nintendo" && next1 == Some("3DS") {
                result.add_tag("NetFront NX", None);
                win.slide_window_by(1);
            }
            // Skip
            else if curr == "like" || curr.len() <= 2 {
                win.slide_window_by(1);
                continue;
            }
            // Rest
            else {
                let parts = curr.split_once(['/', ';', ':']);
                let tag = trim_invalid_chars(parts.map(|(t, _)| t)).unwrap_or(curr);
                let version = trim_invalid_chars(parts.map(|(_, v)| v));

                if tag == "Kindle" {
                    result.add_possible_os_and_browser_tag("Kindle", version);
                }
                //
                else if tag == "FxiOS" {
                    result.add_possible_browser_tag("Firefox iOS", version);
                } else if tag == "EdgiOS" {
                    if let Some(os_token) = result.possible_os_token.as_ref() {
                        if os_token.tag == "Mac OS X" {
                            result.add_possible_os_tag_override_existing("iOS", None);
                        }
                    }
                    result.add_possible_browser_tag("Edge Mobile", version);
                }
                //
                else if tag == "CriOS" {
                    result.ios_hint = true;
                    if let Some(os_token) = result.possible_os_token.as_ref() {
                        if os_token.tag == "Mac OS X" {
                            result.add_possible_os_tag_override_existing("iOS", None);
                        }
                    }
                    result.add_possible_browser_tag("Chrome Mobile iOS", version);
                }
                //
                else if tag == "GSA" {
                    result.add_possible_browser_tag("Google", version);
                }
                //
                else if tag == "YisouSpider" {
                    result.add_possible_browser_tag("YisouSpider", version);
                }
                //
                else if tag == "Edg" || tag == "Edge" {
                    result.add_tag("Edge", version);
                }
                //
                else if tag == "OPR" {
                    result.add_tag("Opera", version);
                }
                //
                else if tag == "SamsungBrowser" {
                    result.add_possible_browser_tag("Samsung Internet", version);
                }
                //
                else if tag == "HuaweiBrowser" {
                    result.huawei_hint = true;

                    result.add_tag("HuaweiBrowser", version);
                }
                //
                else if tag == "ChatGPT-User" {
                    result.add_possible_browser_tag("ChatGPT-User", version);
                }
                //
                else if tag == "OAI-SearchBot" {
                    result.add_possible_browser_tag("OAI-SearchBot", version);
                }
                //
                else if tag == "NX" {
                    result.add_possible_browser_tag("NetFront NX", version);
                }
                //
                else if tag == "Electron" {
                    result.add_possible_browser_tag("Electron", version);
                }
                // Bot or crawler
                else if tag.contains("Bot")
                    || tag.contains("bot")
                    || tag.contains("crawler")
                    || tag.contains("Crawler")
                {
                    result.add_possible_browser_tag_for_bot(tag, version);
                }
                // Mobile
                else if tag == "Mobile" {
                    result.mobile_hint = true;

                    result.add_tag("Mobile", version);
                }
                // Safari
                else if tag == "Safari" {
                    result.safari_hint = true;

                    result.add_tag("Safari", version);
                } else if tag == "CFNetwork" {
                    result.cfnetwork_hint = true;
                    result.ios_hint = true;
                } else if tag.contains("crawler") || version.is_some_and(|v| v.contains("crawler"))
                {
                    result.crawler_hint = true;
                } else if tag == "OculusBrowser" {
                    // Oculus os is android, but fake to be linux
                    result.add_possible_os_tag_override_existing("Android", None);
                }
                //
                else {
                    result.add_tag(tag, version);
                }
            }

            win.slide_window_by(1);
        }

        result
    }
}

#[derive(Debug, Default)]
pub struct TokenizerResult<'a> {
    pub position: usize,
    pub tokens: Vec<Token<'a>>,
    pub possible_os_token: Option<Token<'a>>,
    pub possible_browser_token: Option<Token<'a>>,

    // Hints
    pub linux_hint: bool,
    pub ios_hint: bool,
    pub macos_hint: bool,
    pub windows_hint: bool,
    pub mobile_hint: bool,
    pub safari_hint: bool,
    pub playstation_hint: bool,
    pub huawei_hint: bool,
    pub cfnetwork_hint: bool,
    pub crawler_hint: bool,
}

impl<'a> TokenizerResult<'a> {
    pub fn add_tag(&mut self, tag: &'a str, version: Option<&'a str>) {
        self.tokens.push(Token {
            position: self.position,
            tag,
            version,
        });
        self.position += 1;
    }

    pub fn add_possible_os_and_browser_tag(&mut self, tag: &'a str, version: Option<&'a str>) {
        self.add_possible_os_tag_impl(tag, version);
        self.add_possible_browser_tag_impl(tag, version);

        self.add_tag(tag, version);
    }

    pub fn add_possible_os_tag(&mut self, tag: &'a str, version: Option<&'a str>) {
        self.add_possible_os_tag_impl(tag, version);
        self.add_tag(tag, version);
    }

    pub fn add_possible_os_tag_override_existing(
        &mut self,
        tag: &'a str,
        version: Option<&'a str>,
    ) {
        self.possible_os_token = Some(Token {
            position: self.position,
            tag,
            version,
        });
    }

    pub fn add_possible_browser_tag(&mut self, tag: &'a str, version: Option<&'a str>) {
        self.add_possible_browser_tag_impl(tag, version);
        self.add_tag(tag, version);
    }

    fn add_possible_os_tag_impl(&mut self, tag: &'a str, version: Option<&'a str>) {
        if self.possible_os_token.is_some() {
            return;
        }

        if version.is_none() {
            return;
        }

        self.possible_os_token = Some(Token {
            position: self.position,
            tag,
            version,
        });
    }

    fn add_possible_browser_tag_for_bot(&mut self, tag: &'a str, version: Option<&'a str>) {
        if self.possible_browser_token.is_some()
            && (tag.contains(".com")
                || tag.contains(".net")
                || tag.contains(".org")
                || tag.contains(".html")
                || tag.contains("http://")
                || tag.contains("https://"))
        {
            return;
        }
        self.possible_browser_token = Some(Token {
            position: self.position,
            tag,
            version,
        });
    }

    fn add_possible_browser_tag_impl(&mut self, tag: &'a str, version: Option<&'a str>) {
        if version.is_none() {
            return;
        }
        if self.possible_browser_token.is_some()
            && (tag.contains(".com")
                || tag.contains(".net")
                || tag.contains(".org")
                || tag.contains(".html")
                || tag.contains("http://")
                || tag.contains("https://"))
        {
            return;
        }

        self.possible_browser_token = Some(Token {
            position: self.position,
            tag,
            version,
        });
    }
}

#[derive(Debug, Clone)]
pub struct Token<'a> {
    pub position: usize,
    pub tag: &'a str,
    pub version: Option<&'a str>,
}

impl<'a> Token<'a> {
    pub fn get_version(&self) -> Option<Version<'a>> {
        let version = unwrap_or_return!(self.version, Some(Version::major("0.0.0")));

        if self.tag == "Windows" {
            let mapped = match version {
                "5.1" => "XP",
                "5.2" => "XP",
                "6.0" => "Vista",
                "6.1" => "7", // lol
                "6.3" => "8.1",
                "10.0" => "10",
                _ => "0.0.0",
            };

            return Some(Version::major(mapped));
        }

        let mut parts = version.split_terminator(['_', ',', '.']);

        let mut version = Version::default();
        if let Some(major) = parts.next() {
            version.major = Some(major);
        }

        if let Some(minor) = parts.next() {
            let trimmed_minor = take_until_non_numeric(minor);
            version.minor = Some(trimmed_minor);
        }

        if let Some(patch) = parts.next() {
            version.patch = Some(patch);
        }

        // odd: Don't include patch_minor here
        if self.tag == "YaBrowser" || self.tag == "Opera" || self.tag == "NetFront NX" {
            return Some(version);
        }

        if let Some(patch_minor) = parts.next() {
            version.patch_minor = Some(patch_minor);
        }

        Some(version)
    }
}

fn trim_invalid_chars(s: Option<&str>) -> Option<&str> {
    let trimmed = s.map(|s| {
        s.trim_matches(|c| c == '(' || c == ')' || c == ';' || c == ',' || c == '+' || c == '_')
    });

    match trimmed {
        Some("") => None,
        Some(s) => Some(s),
        None => None,
    }
}

fn starts_with_number(s: Option<&str>) -> bool {
    s.map(|s| s.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .unwrap_or(false)
}

fn consume_if_numeric<'a>(win: &mut WindowIter<'a>, tag: Option<&'a str>) -> Option<&'a str> {
    if starts_with_number(tag) {
        win.slide_window_by(1);
        return tag;
    }

    None
}

fn take_until_non_numeric(s: &str) -> &str {
    let mut slice_index = 0;

    for c in s.chars() {
        if !c.is_ascii_digit() {
            break;
        }

        slice_index += 1;
    }

    s.get(..slice_index).unwrap_or(s)
}
