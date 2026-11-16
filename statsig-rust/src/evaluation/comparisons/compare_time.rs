use crate::{evaluation::evaluator_value::MemoizedEvaluatorValue, unwrap_or_return, DynamicValue};
use chrono::Duration;

pub(crate) fn compare_time(left: &DynamicValue, right: &MemoizedEvaluatorValue, op: &str) -> bool {
    let raw_left_ts = unwrap_or_return!(left.timestamp_value.or(left.int_value), false);
    let left_ts = to_millis(raw_left_ts);

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

fn to_millis(ts: i64) -> i64 {
    // calculate the absolute value of the digits
    let digits = ts.abs();

    // epoch in seconds (milliseconds would be before 1970)
    if digits < 10_000_000_000 {
        // seconds to milliseconds
        ts.saturating_mul(1000)
    } else {
        // milliseconds return directly
        ts
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
    fn test_rfc3339_fractional_seconds_and_offsets() {
        for (timestamp, expected_millis) in [
            ("2026-11-13T00:00:00.123Z", 1794528000123_i64),
            ("2026-11-13T00:00:00+00:00", 1794528000000),
            ("2026-11-13T05:30:00.123+05:30", 1794528000123),
            ("2026-11-12T16:00:00.123456789-08:00", 1794528000123),
        ] {
            let value = create_str_value(timestamp);
            let before = test_only_make_eval_value!(1794527999999_i64);
            let after = test_only_make_eval_value!(1794528001000_i64);
            let same_day = test_only_make_eval_value!(1794528000000_i64);

            assert_eq!(value.timestamp_value, Some(expected_millis), "{timestamp}");
            assert!(compare_time(&value, &before, "after"), "{timestamp}");
            assert!(compare_time(&value, &after, "before"), "{timestamp}");
            assert!(compare_time(&value, &same_day, "on"), "{timestamp}");
            assert!(!compare_time(&value, &after, "after"), "{timestamp}");
        }
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
        for timestamp in [
            "invalid-date",
            "2026-11-13T00:00:00.123+25:00",
            "2026-11-13T00:00:00.123456789-08:00junk",
        ] {
            let value = create_str_value(timestamp);
            assert_eq!(value.timestamp_value, None, "{timestamp}");
            assert!(!compare_time(&value, &test_eval_value, "before"));
        }
    }
}
