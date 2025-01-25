use napi_derive::napi;

#[napi(object)]
pub struct StatsigOptions {
    pub output_log_level: Option<String>,
}

impl From<StatsigOptions> for sigstat::StatsigOptions {
    fn from(options: StatsigOptions) -> Self {
        sigstat::StatsigOptions {
            output_log_level: options.output_log_level.map(|s| s.as_str().into()),
            ..Default::default()
        }
    }
}
