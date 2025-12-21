use rustler::NifStruct;
use serde::Serialize;
use statsig_rust::statsig_options::StatsigOptions as StatsigOptionsActual;

#[derive(NifStruct, Serialize)]
#[module = "Statsig.Options.ProxyConfig"]
pub struct ProxyConfig {
    pub proxy_host: Option<String>,
    pub proxy_port: Option<u16>,
    pub proxy_auth: Option<String>,
    pub proxy_protocol: Option<String>,
}

#[derive(NifStruct, Serialize)]
#[module = "Statsig.Options"]
pub struct StatsigOptions {
    pub environment: Option<String>,
    pub output_log_level: Option<String>,
    pub init_timeout_ms: Option<u64>,
    pub fallback_to_statsig_api: Option<bool>,

    pub log_event_url: Option<String>,
    pub event_logging_flush_interval_ms: Option<u32>,
    pub event_logging_max_queue_size: Option<u32>,

    pub specs_sync_interval_ms: Option<u32>,
    pub specs_url: Option<String>,

    pub enable_id_lists: Option<bool>,
    pub id_lists_sync_interval_ms: Option<u32>,
    pub id_lists_url: Option<String>,

    pub disable_all_logging: Option<bool>,
    pub disable_country_lookup: Option<bool>,
    pub disable_network: Option<bool>, // Disable all out-going network including get configs, log_events...

    pub wait_for_country_lookup_init: Option<bool>,
    pub wait_for_user_agent_init: Option<bool>,

    pub use_third_party_ua_parser: Option<bool>,
    pub disable_disk_access: Option<bool>,

    pub proxy_config: Option<ProxyConfig>,
}

impl From<StatsigOptions> for StatsigOptionsActual {
    fn from(config: StatsigOptions) -> Self {
        StatsigOptionsActual {
            environment: config.environment,
            enable_id_lists: config.enable_id_lists,
            id_lists_url: config.id_lists_url,
            event_logging_max_queue_size: config.event_logging_max_queue_size,
            fallback_to_statsig_api: config.fallback_to_statsig_api,
            specs_sync_interval_ms: config.specs_sync_interval_ms,
            specs_url: config.specs_url,
            log_event_url: config.log_event_url,
            output_log_level: config.output_log_level.map(|o| o.as_str().into()),
            disable_all_logging: config.disable_all_logging,
            disable_country_lookup: config.disable_country_lookup,
            disable_network: config.disable_network,
            wait_for_country_lookup_init: config.wait_for_country_lookup_init,
            wait_for_user_agent_init: config.wait_for_user_agent_init,
            use_third_party_ua_parser: config.use_third_party_ua_parser,
            disable_disk_access: config.disable_disk_access,
            id_lists_sync_interval_ms: config.id_lists_sync_interval_ms,
            #[allow(deprecated)]
            event_logging_flush_interval_ms: config.event_logging_flush_interval_ms,
            init_timeout_ms: config.init_timeout_ms,
            proxy_config: config.proxy_config.map(|p| {
                statsig_rust::networking::proxy_config::ProxyConfig {
                    proxy_host: p.proxy_host,
                    proxy_port: p.proxy_port,
                    proxy_auth: p.proxy_auth,
                    proxy_protocol: p.proxy_protocol,
                }
            }),
            ..StatsigOptionsActual::default()
        }
    }
}
