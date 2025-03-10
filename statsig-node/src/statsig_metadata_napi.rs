use napi::Env;
use statsig_rust::statsig_metadata::StatsigMetadata;

const UNKNOWN_ENV: &str = "unknown";

pub(crate) fn update_statsig_metadata(env: Option<Env>) {
    StatsigMetadata::update_values(
        "statsig-server-core-node".to_string(),
        get_os(),
        get_arch(),
        get_node_version(env),
    );
}

fn get_node_version(env: Option<Env>) -> String {
    let env = match env {
        Some(env) => env,
        None => return UNKNOWN_ENV.to_string(),
    };

    if let Ok(node_version) = env.get_node_version() {
        format!(
            "{}.{}.{}",
            node_version.major, node_version.minor, node_version.patch
        )
    } else {
        UNKNOWN_ENV.to_string()
    }
}

fn get_os() -> String {
    let os = std::env::consts::OS;
    os.to_string()
}

fn get_arch() -> String {
    let arch = std::env::consts::ARCH;
    arch.to_string()
}
