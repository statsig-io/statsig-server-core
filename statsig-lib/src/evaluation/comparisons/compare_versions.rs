use crate::{unwrap_or_return, DynamicValue};

pub(crate) fn compare_versions(left: &DynamicValue, right: &DynamicValue, op: &str) -> bool {
    let left_str = unwrap_or_return!(&left.string_value, false);
    let right_str = unwrap_or_return!(&right.string_value, false);

    fn comparison(left_str: &str, right_str: &str) -> i32 {
        let left_version = left_str.split('-').next().unwrap_or("");
        let right_version = right_str.split('-').next().unwrap_or("");

        let mut left_parts = left_version.split('.');
        let mut right_parts = right_version.split('.');

        loop {
            let opt_left_num = left_parts.next().and_then(|s| s.parse::<i32>().ok());
            let opt_right_num = right_parts.next().and_then(|s| s.parse::<i32>().ok());

            // If both iterators are exhausted, we break the loop
            if opt_left_num.is_none() && opt_right_num.is_none() {
                break;
            }

            let left_num = opt_left_num.unwrap_or_default();
            let right_num = opt_right_num.unwrap_or_default();

            if left_num < right_num {
                return -1;
            }

            if left_num > right_num {
                return 1;
            }
        }

        0
    }

    let result = comparison(left_str, right_str);

    match op {
        "version_gt" => result > 0,
        "version_gte" => result >= 0,
        "version_lt" => result < 0,
        "version_lte" => result <= 0,
        "version_eq" => result == 0,
        "version_neq" => result != 0,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::evaluation::comparisons::compare_versions;
    use crate::{dyn_value, DynamicValue};

    #[test]
    fn test_version_comparison_equal() {
        let left = dyn_value!("1.2.3");
        let right = dyn_value!("1.2.3");

        let result = compare_versions(&left, &right, "version_eq");
        assert!(result);
    }

    #[test]
    fn test_version_comparison_greater_than() {
        let left = dyn_value!("1.2.4");
        let right = dyn_value!("1.2.3");

        let result = compare_versions(&left, &right, "version_gt");
        assert!(result);
    }

    #[test]
    fn test_version_comparison_less_than() {
        let left = dyn_value!("1.2.3");
        let right = dyn_value!("1.2.4");

        let result = compare_versions(&left, &right, "version_lt");
        assert!(result);
    }
}
