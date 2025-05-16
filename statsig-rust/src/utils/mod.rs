#[cfg(target_env = "gnu")]
use crate::log_d;

// Manually free memory
#[cfg(target_env = "gnu")]
extern "C" {
    fn malloc_trim(pad: libc::size_t) -> libc::c_int;
}

#[cfg(target_env = "gnu")]
pub fn maybe_trim_malloc() {
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
pub fn maybe_trim_malloc() {
    // No-op only glibc supports malloc_trim function
}

pub fn get_api_from_url(url: &str) -> String {
    let path_start = url.find("://").and_then(|scheme_end| {
        let after_scheme = &url[scheme_end + 3..];
        after_scheme
            .find('/')
            .map(|slash_pos| scheme_end + 3 + slash_pos)
    });

    if let Some(path_pos) = path_start {
        url[..path_pos].to_string()
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_network_api() {
        // Test with full URL containing path
        let api = get_api_from_url("http://localhost:8080/v1/endpoint");
        assert_eq!(api, "http://localhost:8080".to_string());

        // Test with URL without path
        let api = get_api_from_url("http://localhost:8080");
        assert_eq!(api, "http://localhost:8080".to_string());

        // Test with HTTPS URL
        let api = get_api_from_url("https://api.example.com/v1/specs");
        assert_eq!(api, "https://api.example.com".to_string());

        // Test with empty string
        let api = get_api_from_url("");
        assert_eq!(api, "".to_string());
    }
}
