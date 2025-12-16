mod utils;

use more_asserts::assert_gt;
use statsig_rust::{
    evaluation::{dynamic_string::DynamicString, user_agent_parsing::UserAgentParser},
    interned_string::InternedString,
    user::StatsigUserInternal,
    StatsigUser,
};
use utils::helpers::load_contents;

lazy_static::lazy_static! {
    static ref TEST_CASES: Vec<TestCase> = load_contents("test_user_agents.txt")
        .lines()
        .skip(1) // skip header
        .map(parse_test_case)
        .collect();
}

// See thirdparty baseline in this PR: https://github.com/statsig-io/private-statsig-server-core/pull/2498

// statsig-uaparser correctness against Python's 'ua-parser'
const OS_NAME_THRESHOLD: f64 = 0.87;
const OS_VERSION_THRESHOLD: f64 = 0.84;

// todo: bring this up to 1.0
const BROWSER_NAME_THRESHOLD: f64 = 0.41;
const BROWSER_VERSION_THRESHOLD: f64 = 0.31;

const LOG_FAILURES: bool = false;

fn log_failure(test_case: &TestCase, field: &str, expected: Option<&str>, got: Option<&str>) {
    if !LOG_FAILURES {
        return;
    }

    println!(
        "Field: {}\n Expected: \"{}\"\n Got: \"{}\"\n UA: \"{}\"\n",
        field,
        expected.unwrap_or_default(),
        got.unwrap_or_default(),
        test_case.user_agent
    );
}

#[test]
fn test_user_agent_parser_os_name() {
    let mut hit = 0;
    let mut miss = 0;

    for test_case in TEST_CASES.iter() {
        let name = extract_field_from_user_agent(&test_case.user_agent, "os_name");

        if name.as_deref() == test_case.expected_os_family.as_deref() {
            hit += 1;
        } else {
            miss += 1;
            log_failure(
                test_case,
                "os_name",
                test_case.expected_os_family.as_deref(),
                name.as_deref(),
            );
        }
    }

    let score = hit as f64 / (hit + miss) as f64;
    assert_gt!(score, OS_NAME_THRESHOLD);
}

#[test]
fn test_user_agent_parser_os_version() {
    let mut hit = 0;
    let mut miss = 0;

    for test_case in TEST_CASES.iter() {
        let version = extract_field_from_user_agent(&test_case.user_agent, "os_version");

        if version.as_deref() == test_case.expected_os_version.as_deref() {
            hit += 1;
        } else {
            miss += 1;
            log_failure(
                test_case,
                "os_version",
                test_case.expected_os_version.as_deref(),
                version.as_deref(),
            );
        }
    }

    let score = hit as f64 / (hit + miss) as f64;
    assert_gt!(score, OS_VERSION_THRESHOLD);
}

#[test]
fn test_user_agent_parser_browser_name() {
    let mut hit = 0;
    let mut miss = 0;

    for test_case in TEST_CASES.iter() {
        let name = extract_field_from_user_agent(&test_case.user_agent, "browser_name");

        if name.as_deref() == test_case.expected_browser_family.as_deref() {
            hit += 1;
        } else {
            miss += 1;
            log_failure(
                test_case,
                "browser_name",
                test_case.expected_browser_family.as_deref(),
                name.as_deref(),
            );
        }
    }

    let score = hit as f64 / (hit + miss) as f64;
    assert_gt!(score, BROWSER_NAME_THRESHOLD);
}

#[test]
fn test_user_agent_parser_browser_version() {
    let mut hit = 0;
    let mut miss = 0;

    for test_case in TEST_CASES.iter() {
        let version = extract_field_from_user_agent(&test_case.user_agent, "browser_version");

        if version.as_deref() == test_case.expected_browser_version.as_deref() {
            hit += 1;
        } else {
            miss += 1;
            log_failure(
                test_case,
                "browser_version",
                test_case.expected_browser_version.as_deref(),
                version.as_deref(),
            );
        }
    }

    let score = hit as f64 / (hit + miss) as f64;
    assert_gt!(score, BROWSER_VERSION_THRESHOLD);
}

fn extract_field_from_user_agent(user_agent: &str, field: &str) -> Option<InternedString> {
    let mut user = StatsigUser::with_user_id("");
    user.set_user_agent(user_agent.to_string());
    let user_internal = StatsigUserInternal::new(&user, None);

    let mut dummy_override_reason = None;
    let field = DynamicString::from(field.to_string());

    let result = UserAgentParser::get_value_from_user_agent(
        &user_internal,
        &Some(field),
        &mut dummy_override_reason,
        /* use_third_party_ua_parser */ false,
    );

    match result {
        Some(value) => value.string_value.map(|s| s.value.clone()),
        None => None,
    }
}

#[derive(Default)]
pub struct TestCase {
    pub user_agent: String,
    pub expected_os_family: Option<String>,
    pub expected_os_version: Option<String>,
    pub expected_browser_family: Option<String>,
    pub expected_browser_version: Option<String>,
}

fn parse_test_case(test_case_data: &str) -> TestCase {
    let parts = test_case_data.split("|").collect::<Vec<&str>>();

    let mut result = TestCase {
        user_agent: parts[0].to_string(),
        ..Default::default()
    };

    if parts[1] != "None" {
        result.expected_os_family = Some(parts[1].to_string());
    } else {
        result.expected_os_family = Some("Other".to_string());
    }

    if let Some(os_version) = create_version(parts[2], parts[3], parts[4]) {
        result.expected_os_version = Some(os_version);
    }

    // skip patch_minor index(5)

    if parts[6] != "None" {
        result.expected_browser_family = Some(parts[6].to_string());
    } else {
        result.expected_browser_family = Some("Other".to_string());
    }

    if let Some(browser_version) = create_version(parts[7], parts[8], parts[9]) {
        result.expected_browser_version = Some(browser_version);
    }

    result
}

fn create_version(major: &str, minor: &str, patch: &str) -> Option<String> {
    if major == "None" {
        return Some("0.0.0".to_string());
    }

    let mut version = String::new();

    if major != "None" {
        version.push_str(major);
    }

    if minor != "None" {
        version.push('.');
        version.push_str(minor);
    }

    if patch != "None" {
        version.push('.');
        version.push_str(patch);
    }

    Some(version)
}
