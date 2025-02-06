use napi_derive::*;

#[napi(object)]
pub struct StatsigResult {
    pub is_success: bool,
    pub error: Option<String>,
}

impl StatsigResult {
    pub fn success() -> Self {
        Self {
            is_success: true,
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            is_success: false,
            error: Some(error),
        }
    }
}
