use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsigMetadata {
    pub sdk_type: String,
    pub sdk_version: String,
}

impl StatsigMetadata {
    pub fn new() -> Self {
        Self {
            sdk_version: "0.0.1".to_string(),
            sdk_type: "statsig-server-core".to_string(),
        }
    }
}
