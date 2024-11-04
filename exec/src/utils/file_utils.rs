use std::fs;
use std::path::Path;

pub fn ensure_empty_dir(dir: &str) {
    let path = Path::new(dir);

    if path.exists() {
        fs::remove_dir_all(path).expect("Failed to remove directory");
    }

    fs::create_dir_all(path).expect("Failed to create directory");
}
