use crate::{unwrap_or_return, DynamicValue};
use chrono::Duration;

pub(crate) fn compare_time(left: &DynamicValue, right: &DynamicValue, op: &str) -> bool {
    let left_num = unwrap_or_return!(left.int_value, false);
    let right_num = unwrap_or_return!(right.int_value, false);

    match op {
        "before" => left_num < right_num,
        "after" => left_num > right_num,
        "on" => {
            Duration::milliseconds(left_num).num_days()
                == Duration::milliseconds(right_num).num_days()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::evaluation::comparisons::compare_time;
    use crate::DynamicValue;
    use chrono::Utc;

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
}
