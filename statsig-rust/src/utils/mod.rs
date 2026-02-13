#[cfg(target_env = "gnu")]
use crate::log_d;

// Manually free memory
#[cfg(target_env = "gnu")]
extern "C" {
    fn malloc_trim(pad: libc::size_t) -> libc::c_int;
}

#[cfg(target_env = "gnu")]
pub fn try_release_unused_heap_memory() {
    // Glibc requested more memory than needed when deserializing a big json blob
    // And memory allocator fails to return it.
    // To prevent service from OOMing, manually unused heap memory.

    unsafe {
        // Free as much memory as possible
        let result = malloc_trim(0);
        if result == 0 {
            log_d!("MemoryUtils", "No memory was released by malloc_trim.");
        } else {
            log_d!("MemoryUtils", "Memory was released by malloc_trim.");
        }
    }
}
#[cfg(not(target_env = "gnu"))]
pub fn try_release_unused_heap_memory() {
    // No-op only glibc supports malloc_trim function
}

// used for logging into metrics / diagnostics
pub fn get_api_from_url(url: &str) -> String {
    // 1) Split into base
    let (base, path) = match url.find("://") {
        Some(scheme_end) => {
            let after_scheme = &url[scheme_end + 3..];
            match after_scheme.find('/') {
                Some(slash) => (
                    &url[..scheme_end + 3 + slash],
                    &url[scheme_end + 3 + slash..],
                ),
                None => (url, ""),
            }
        }
        None => (url, ""), // not a full URL; treat as base-only
    };

    // 2) If path begins with "/v<digits>" and next is "/" or end, return base + "/v<digits>"
    if let Some(rest) = path.strip_prefix("/v") {
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() {
            let after = &rest[digits.len()..];
            if after.is_empty() || after.starts_with('/') {
                return format!("{base}/v{digits}");
            }
        }
    }

    base.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_network_api() {
        // Test with full URL containing path
        let api = get_api_from_url("http://localhost:8080/v1/endpoint");
        assert_eq!(api, "http://localhost:8080/v1".to_string());

        // Test with v2 path
        let api = get_api_from_url("http://localhost:8080/v2/download_config_specs");
        assert_eq!(api, "http://localhost:8080/v2".to_string());

        // Test with URL without path
        let api = get_api_from_url("http://localhost:8080");
        assert_eq!(api, "http://localhost:8080".to_string());

        // Test with HTTPS URL
        let api = get_api_from_url("https://api.example.com/v1/specs");
        assert_eq!(api, "https://api.example.com/v1".to_string());

        let api = get_api_from_url("https://api.statsig.com/v1/get_id_lists");
        assert_eq!(api, "https://api.statsig.com/v1".to_string());

        let api = get_api_from_url("https://api.statsigcdn.com/v1/download_id_list_file");
        assert_eq!(api, "https://api.statsigcdn.com/v1".to_string());

        let api = get_api_from_url("https://api.statsig.com/get_id_lists");
        assert_eq!(api, "https://api.statsig.com".to_string());

        // Test with larger version
        let api = get_api_from_url("https://api.example.com/v10/specs");
        assert_eq!(api, "https://api.example.com/v10".to_string());

        // Test with version suffix (should not truncate to /v1)
        let api = get_api_from_url("https://api.example.com/v1beta/specs");
        assert_eq!(api, "https://api.example.com".to_string());

        // Test with query string
        let api = get_api_from_url("https://api.example.com/v1/specs?x=1");
        assert_eq!(api, "https://api.example.com/v1".to_string());

        // Test with fragment
        let api = get_api_from_url("https://api.example.com/v1/specs#frag");
        assert_eq!(api, "https://api.example.com/v1".to_string());

        // Test with nested version segments
        let api = get_api_from_url("https://api.example.com/v1/foo/v2/bar");
        assert_eq!(api, "https://api.example.com/v1".to_string());

        // Test with IPv6 host
        let api = get_api_from_url("http://[::1]:8080/v1/specs");
        assert_eq!(api, "http://[::1]:8080/v1".to_string());

        // Test with trailing slash
        let api = get_api_from_url("https://api.example.com/v1/");
        assert_eq!(api, "https://api.example.com/v1".to_string());

        // Test with empty string
        let api = get_api_from_url("");
        assert_eq!(api, "".to_string());
    }
}
