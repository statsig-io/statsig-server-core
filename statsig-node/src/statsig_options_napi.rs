use napi_derive::napi;

#[napi(object)]
pub struct StatsigOptions {
    pub specs_url: Option<String>,
    pub log_event_url: Option<String>,
    pub output_log_level: Option<String>,
}

impl From<StatsigOptions> for sigstat::StatsigOptions {
    fn from(options: StatsigOptions) -> Self {
        sigstat::StatsigOptions {
            output_log_level: options.output_log_level.map(|s| s.as_str().into()),
            specs_url: options.specs_url,
            log_event_url: options.log_event_url,
            ..Default::default()
        }
    }
}
