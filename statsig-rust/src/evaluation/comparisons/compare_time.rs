use crate::{evaluation::evaluator_value::EvaluatorValue, unwrap_or_return, DynamicValue};
use chrono::Duration;

pub(crate) fn compare_time(left: &DynamicValue, right: &EvaluatorValue, op: &str) -> bool {
    let raw_left_ts = unwrap_or_return!(left.timestamp_value.or(left.int_value), false);
    let mut left_ts = raw_left_ts;
    if left_ts < 1_000_000_000_000 {
        // Assume left is in seconds, convert to milliseconds
        left_ts *= 1000;
    }

    // dcs will always be in milliseconds
    let right_ts = unwrap_or_return!(
        right
            .timestamp_value
            .or(right.float_value.map(|x| x as i64)),
        false
    );

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
    use crate::{dyn_value, test_only_make_eval_value, DynamicValue};
    use chrono::Utc;

    fn create_str_value(s: &str) -> DynamicValue {
        dyn_value!(s.to_string())
    }

    #[test]
    fn test_compare_before() {
        let now = Utc::now().timestamp_millis();
        let left = DynamicValue::for_timestamp_evaluation(now);
        let right = test_only_make_eval_value!(now + 1);

        assert!(compare_time(&left, &right, "before"));
    }

    #[test]
    fn test_compare_after() {
        let now = Utc::now().timestamp_millis();
        let left = DynamicValue::for_timestamp_evaluation(now + 1);
        let right = test_only_make_eval_value!(now);

        assert!(compare_time(&left, &right, "after"));
    }

    #[test]
    fn test_rfc3339_format() {
        let test_eval_value = test_only_make_eval_value!("2023-01-02T00:00:00Z");
        assert!(compare_time(
            &create_str_value("2023-01-01T00:00:00Z"),
            &test_eval_value,
            "before"
        ));
    }

    #[test]
    fn test_custom_datetime_format() {
        let test_eval_value = test_only_make_eval_value!("2023-01-02 00:00:00");
        assert!(compare_time(
            &create_str_value("2023-01-01 00:00:00"),
            &test_eval_value,
            "before"
        ));
    }

    #[test]
    fn test_iso_format_no_separator() {
        let test_eval_value = test_only_make_eval_value!("2023-01-02 00:00:00Z");
        assert!(compare_time(
            &create_str_value("2023-01-01 00:00:00Z"),
            &test_eval_value,
            "before"
        ));
    }

    #[test]
    fn test_timestamp_string() {
        let test_eval_value = test_only_make_eval_value!("1672617600000");
        assert!(compare_time(
            &create_str_value("1672531200000"), // 2023-01-01
            &test_eval_value,                   // 2023-01-02
            "before"
        ));
    }

    #[test]
    fn test_mixed_formats() {
        let test_eval_value = test_only_make_eval_value!(1672617600000_i64);
        assert!(compare_time(
            &create_str_value("2023-01-01T00:00:00Z"),
            &test_eval_value, // 2023-01-02
            "before"
        ));
    }

    #[test]
    fn test_invalid_input() {
        let test_eval_value = test_only_make_eval_value!(1672617600000_i64);
        assert!(!compare_time(
            &create_str_value("invalid-date"),
            &test_eval_value,
            "before"
        ));
    }
}
