use serde_json::{Map, Value};

pub fn enforce_array(value: &Value) -> Vec<Value> {
    value.as_array().unwrap().clone()
}

pub fn enforce_object(value: &Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

pub fn enforce_string(value: &Value) -> String {
    value.as_str().unwrap().to_string()
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
