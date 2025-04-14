use crate::DynamicValue;
use chrono::Duration;

pub(crate) fn compare_time(left: &DynamicValue, right: &DynamicValue, op: &str) -> bool {
    let get_timestamp = |dv: &DynamicValue| -> Option<i64> { dv.timestamp_value.or(dv.int_value) };

    let (Some(left_ts), Some(right_ts)) = (get_timestamp(left), get_timestamp(right)) else {
        return false;
    };

    match op {
        "before" => left_ts < right_ts,
        "after" => left_ts > right_ts,
        "on" => {
            Duration::milliseconds(left_ts).num_days()
                == Duration::milliseconds(right_ts).num_days()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::evaluation::comparisons::compare_time;
    use crate::DynamicValue;
    use chrono::Utc;

    fn create_int_value(ts: i64) -> DynamicValue {
        DynamicValue {
            int_value: Some(ts),
            ..Default::default()
        }
    }

    fn create_str_value(s: &str) -> DynamicValue {
        DynamicValue::from(s.to_string())
    }

    #[test]
    fn test_compare_before() {
        let now = Utc::now().timestamp_millis();
        let left = DynamicValue::for_timestamp_evaluation(now);
        let right = DynamicValue::for_timestamp_evaluation(now + 1);

        assert!(compare_time(&left, &right, "before"));
    }

    #[test]
    fn test_compare_after() {
        let now = Utc::now().timestamp_millis();
        let left = DynamicValue::for_timestamp_evaluation(now + 1);
        let right = DynamicValue::for_timestamp_evaluation(now);

        assert!(compare_time(&left, &right, "after"));
    }

    #[test]
    fn test_rfc3339_format() {
        assert!(compare_time(
            &create_str_value("2023-01-01T00:00:00Z"),
            &create_str_value("2023-01-02T00:00:00Z"),
            "before"
        ));
    }

    #[test]
    fn test_custom_datetime_format() {
        assert!(compare_time(
            &create_str_value("2023-01-01 00:00:00"),
            &create_str_value("2023-01-02 00:00:00"),
            "before"
        ));
    }

    #[test]
    fn test_timestamp_string() {
        assert!(compare_time(
            &create_str_value("1672531200000"), // 2023-01-01
            &create_str_value("1672617600000"), // 2023-01-02
            "before"
        ));
    }

    #[test]
    fn test_mixed_formats() {
        assert!(compare_time(
            &create_str_value("2023-01-01T00:00:00Z"),
            &create_int_value(1672617600000), // 2023-01-02
            "before"
        ));
    }

    #[test]
    fn test_invalid_input() {
        assert!(!compare_time(
            &create_str_value("invalid-date"),
            &create_int_value(1672617600000),
            "before"
        ));
    }
}
