use std::fmt::{self, Display};

pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub beta: String,
}

impl SemVer {
    pub fn from_string(version: &str) -> SemVer {
        let version_parts: Vec<&str> = version.split('-').next().unwrap().split('.').collect();

        let major = version_parts[0]
            .parse::<u32>()
            .expect("Failed to parse major version");

        let minor = version_parts[1]
            .parse::<u32>()
            .expect("Failed to parse minor version");

        let patch = version_parts[2]
            .parse::<u32>()
            .expect("Failed to parse patch version");

        let beta = version.split('-').nth(1).unwrap_or("").to_string();

        SemVer {
            major,
            minor,
            patch,
            beta,
        }
    }
}

impl Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.beta.is_empty() {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        } else {
            write!(
                f,
                "{}.{}.{}-{}",
                self.major, self.minor, self.patch, self.beta
            )
        }
    }
}

pub fn get_cargo_toml() -> toml::Value {
    let contents = std::fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    toml::from_str(&contents).expect("Failed to parse Cargo.toml")
}

pub fn get_cargo_toml_version() -> SemVer {
    let toml = get_cargo_toml();
    let version = toml["workspace"]["package"]["version"]
        .as_str()
        .expect("Failed to get version from Cargo.toml");

    SemVer::from_string(version)
}

pub fn write_version_to_cargo_toml(version: &SemVer) {
    let mut cargo_toml = get_cargo_toml();

    cargo_toml["workspace"]["package"]["version"] = toml::Value::String(version.to_string());

    let toml_string = toml::to_string(&cargo_toml).expect("Failed to serialize TOML");
    std::fs::write("Cargo.toml", toml_string).expect("Failed to write to Cargo.toml");
}
