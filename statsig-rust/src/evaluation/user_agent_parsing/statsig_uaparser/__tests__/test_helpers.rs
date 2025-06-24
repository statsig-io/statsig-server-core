use std::{fs, path::PathBuf};

pub fn load_test_resource(resource_file_name: &str) -> String {
    let base_path = env!("CARGO_MANIFEST_DIR");
    let path = PathBuf::from(format!("{base_path}/{resource_file_name}"));
    println!("path: {:?}", path);
    fs::read_to_string(path).expect("Unable to read resource file")
}

pub fn load_test_cases_from_file(file_name: &str) -> Vec<TestCase> {
    let contents = load_test_resource(file_name);
    contents.lines().skip(1).map(parse_test_case).collect()
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
    }

    if let Some(os_version) = create_version(parts[2], parts[3], parts[4], parts[5]) {
        result.expected_os_version = Some(os_version);
    }

    if parts[6] != "None" {
        result.expected_browser_family = Some(parts[6].to_string());
    }

    if let Some(browser_version) = create_version(parts[7], parts[8], parts[9], parts[10]) {
        result.expected_browser_version = Some(browser_version);
    }

    result
}

fn create_version(major: &str, minor: &str, patch: &str, patch_minor: &str) -> Option<String> {
    if major == "None" {
        return None;
    }

    let mut version = String::new();
    version.push_str(major);

    if minor != "None" {
        version.push('.');
        version.push_str(minor);
    }

    if patch != "None" {
        version.push('.');
        version.push_str(patch);
    }

    if patch_minor != "None" {
        version.push('.');
        version.push_str(patch_minor);
    }

    Some(version)
}
