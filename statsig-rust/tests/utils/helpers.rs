use serde_json::{Map, Value};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

pub fn enforce_array(value: &Value) -> Vec<Value> {
    value.as_array().unwrap().clone()
}

pub fn enforce_object(value: &Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

pub fn enforce_string(value: &Value) -> String {
    value.as_str().unwrap().to_string()
}

pub fn load_contents(resource_file_name: &str) -> String {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(format!("tests/data/{resource_file_name}"));
    fs::read_to_string(path).expect("Unable to read resource file")
}

pub async fn assert_eventually<F>(assertion: F, timeout_ms: Duration)
where
    F: Fn() -> bool,
{
    let steps = timeout_ms.as_millis() / 10;
    for _ in 0..steps {
        if assertion() {
            return;
        }

        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    panic!("assertion timed out");
}

#[macro_export]
macro_rules! assert_eventually_eq {
    ($actual:expr, $expected:expr, $timeout_ms:expr) => {
        async {
            let start = Instant::now();
            let mut actual_value = $actual();
            while start.elapsed().as_millis() < $timeout_ms.as_millis() {
                if actual_value == $expected {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
                actual_value = $actual();
            }

            if actual_value != $expected {
                panic!("actual {} != expected {}", actual_value, $expected);
            }
        }
    };
}
