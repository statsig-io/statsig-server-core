pub fn sanitize_url_for_logging(url: &str) -> String {
    let secret_key_idx = url.find("secret-");
    let end_idx = url.find(".json").map(|idx| idx + 5).unwrap_or(url.len());
    if let Some(idx) = secret_key_idx {
        let mut sanitized = url.to_string();
        sanitized.replace_range(idx + 12..end_idx, "*****");
        return sanitized;
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::sanitize_url_for_logging;

    #[test]
    fn test_sanitize_url_for_logging() {
        let test_cases = HashMap::from(
            [
                ("https://api.statsigcdn.com/v2/download_config_specs/secret-jadkfjalkjnsdlvcnjsdfaf.json", "https://api.statsigcdn.com/v2/download_config_specs/secret-jadkf*****"),
                ("https://api.statsigcdn.com/v1/log_event/","https://api.statsigcdn.com/v1/log_event/"),
                ("https://api.statsigcdn.com/v2/download_config_specs/secret-jadkfjalkjnsdlvcnjsdfaf.json?sinceTime=1", "https://api.statsigcdn.com/v2/download_config_specs/secret-jadkf*****?sinceTime=1"),
            ]
        );
        for (before, expected) in test_cases {
            let sanitized = sanitize_url_for_logging(before);
            assert!(sanitized == expected)
        }
    }
}
