use std::{collections::HashMap, fs, sync::Arc};

use statsig_rust::{Statsig, StatsigOptions, StatsigUser};

fn load_data() -> HashMap<String, HashMap<String, String>> {
    let base_path = env!("CARGO_MANIFEST_DIR");
    let s = fs::read_to_string(format!("{base_path}/tests/data/ua_string_cases.json")).unwrap();
    serde_json::from_str(&s).unwrap()
}

#[tokio::test]
async fn test_experiment_ua_parser() {
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            wait_for_user_agent_init: Some(true),
            __experimental_ua_parsing_enabled: Some(true),
            ..Default::default()
        })),
    );
    let _ = statsig.initialize().await;
    let test_cases = load_data();
    let mut user = StatsigUser::with_user_id("user1".to_string());
    for (ua_string, expected_value) in test_cases {
        user.set_user_agent(ua_string.clone());
        let sdk_ua_value: statsig_rust::evaluation::user_agent_parsing::ParsedUserAgentValue =
            statsig.__get_parsed_user_agent_value(&user).unwrap();
        assert!(sdk_ua_value.os_name.unwrap_or_default() == *expected_value.get("osName").unwrap());
        assert!(sdk_ua_value.os_version.unwrap() == *expected_value.get("osVersion").unwrap());
        assert!(
            sdk_ua_value.browser_name.unwrap_or_default()
                == *expected_value.get("browserName").unwrap()
        );
        assert!(
            sdk_ua_value.browser_version.unwrap() == *expected_value.get("browserVersion").unwrap()
        );
    }
}

#[tokio::test]
async fn test_3rd_party_ua_parser() {
    let statsig = Statsig::new(
        "secret-key",
        Some(Arc::new(StatsigOptions {
            wait_for_user_agent_init: Some(true),
            ..Default::default()
        })),
    );
    let _ = statsig.initialize().await;
    let mut user = StatsigUser::with_user_id("user1".to_string());
    user.set_user_agent("ChatGPT/1.2025.202 (Windows_NT 10.0.22631; x86_64; build ) Electron/37.2.1 Chrome/138.0.7204.97");
    let ua_value = statsig.__get_parsed_user_agent_value(&user).unwrap();
    assert!(ua_value.os_name.unwrap() == "Windows");
    assert!(ua_value.os_version.unwrap() == "0.0.0"); // TODO(xinli): This is probably a bug within evaluator
    assert!(ua_value.browser_name.unwrap() == "Electron");
    assert!(ua_value.browser_version.unwrap() == "37.2.1")
}
