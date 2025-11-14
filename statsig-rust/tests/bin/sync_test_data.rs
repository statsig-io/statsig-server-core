#[tokio::main]
async fn main() {
    // CompanyID: 5NprLGRxV3W28hreG51Z7n
    let eval_proj_key = std::env::var("test_api_key").unwrap();
    download_json_and_proto_for(&eval_proj_key, "eval_proj_dcs").await;

    // CompanyID: 3ncq00rOXvRdibzZTVUDgl
    let perf_sdk_key = std::env::var("PERF_SDK_KEY").unwrap();
    download_json_and_proto_for(&perf_sdk_key, "perf_proj_dcs").await;

    // CompanyID: 2etz0PtUkhGsJfH0mR6whu
    let demo_sdk_key = std::env::var("statsig_demo_server_key").unwrap();
    download_json_and_proto_for(&demo_sdk_key, "demo_proj_dcs").await;
}

async fn download_json_and_proto_for(sdk_key: &str, name: &str) {
    download_from_url_to_file(
        format!("https://staging.statsigapi.net/v2/download_config_specs/{sdk_key}.json"),
        format!("{name}.json"),
    )
    .await;

    download_from_url_to_file(
        format!("https://staging.statsigapi.net/v2/download_config_specs/{sdk_key}.json?supports_proto=1"),
        format!("{name}.pb.br"),
    )
    .await;
}

async fn download_from_url_to_file(url: String, filename: String) {
    let response = reqwest::get(&url).await.unwrap();
    let body = response.text().await.unwrap();
    write_to_data_dir(&filename, &body);
}

fn write_to_data_dir(filename: &str, body: &str) {
    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/data");
    path.push(filename);
    std::fs::write(path, body).unwrap();
}
