//! Manual integration test: point this SDK at a Statsig forward-proxy
//! deployment and optionally enrich users via `POST /v1/user_enrichment`.
//!
//! Required env:
//!   STATSIG_SERVER_SDK_KEY   server secret (must start with `secret-`)
//!   STATSIG_PROXY_URL        proxy base URL, e.g. http://localhost:8000
//!
//! Optional env:
//!   STATSIG_GATE_NAME        gate to check (default: employee_gate)
//!   STATSIG_USER_ID          userID (default: a-user)
//!   STATSIG_EMPLOYEE_ID      customID employeeID (default: e-42)
//!
//! Run:
//!   cargo test -p statsig-rust --test forward_proxy_usage_tests -- --ignored --nocapture

use statsig_rust::experimental::{
    ForwardProxyEnrichmentOptions, ForwardProxyUserEnrichmentProvider, StatsigFPUserBuilder,
};
use statsig_rust::{Statsig, StatsigOptions};
use std::env;
use std::sync::Arc;

fn required_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} must be set"))
}

fn proxy_base_url() -> String {
    required_env("STATSIG_PROXY_URL")
        .trim()
        .trim_end_matches('/')
        .to_string()
}

fn sdk_key() -> String {
    let key = required_env("STATSIG_SERVER_SDK_KEY");
    assert!(
        key.starts_with("secret-"),
        "STATSIG_SERVER_SDK_KEY must be a server secret"
    );
    key
}

#[tokio::test]
#[ignore = "hits a live forward proxy; set STATSIG_SERVER_SDK_KEY and STATSIG_PROXY_URL"]
async fn http_polling_against_forward_proxy() {
    let secret = sdk_key();
    let proxy = proxy_base_url();

    let options = StatsigOptions {
        specs_url: Some(format!("{proxy}/v2/download_config_specs")),
        fallback_to_statsig_api: Some(true),
        enable_id_lists: Some(false),
        ..Default::default()
    };

    let statsig = Statsig::new(&secret, Some(Arc::new(options)));
    statsig.initialize().await.expect("statsig initialize");

    let fp_enricher = Arc::new(
        ForwardProxyUserEnrichmentProvider::new(
            &secret,
            ForwardProxyEnrichmentOptions {
                url: proxy,
                timeout_ms: Some(100),
                retries: Some(1),
                proxy_config: None,
            },
        )
        .expect("enrichment provider"),
    );

    let user = StatsigFPUserBuilder::new_with_enricher(fp_enricher)
        .user_id(Some("000dobcorQ"))
        .build()
        .await
        .expect("enrichment");

    println!(
        "enrichment ok: id_list_memberships={:?}",
        user.data.id_list_memberships
    );

    let gate_name = "test-with-idlist";
    let gate = statsig.get_feature_gate(&user, gate_name);
    println!(
        "gate={gate_name} value={} reason={}",
        gate.value, gate.details.reason
    );

    let gcir = statsig.get_client_init_response(&user);
    println!("gcir feature_gates={}", gcir.feature_gates.len());

    statsig.shutdown().await.expect("statsig shutdown");
}
