use crate::{evaluation::evaluator_value::MemoizedEvaluatorValue, unwrap_or_return, DynamicValue};

pub(crate) fn compare_versions(
    left: &DynamicValue,
    right: &MemoizedEvaluatorValue,
    op: &str,
) -> bool {
    let left_dyn_str = unwrap_or_return!(&left.string_value, false);
    let right_dyn_str = unwrap_or_return!(&right.string_value, false);

    let left_str = &left_dyn_str.value;
    let right_str = &right_dyn_str.value;

    fn comparison(left_str: &str, right_str: &str) -> i32 {
        let left_version = left_str.split('-').next().unwrap_or("");
        let right_version = right_str.split('-').next().unwrap_or("");

        let mut left_parts = left_version.split('.');
        let mut right_parts = right_version.split('.');

        loop {
            let opt_left_part = left_parts.next();
            let opt_right_part = right_parts.next();

            let opt_left_num = match opt_left_part {
                Some(s) => s.trim().parse::<i128>().ok(),
                None => None,
            };
            let opt_right_num = match opt_right_part {
                Some(s) => s.trim().parse::<i128>().ok(),
                None => None,
            };

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
    use crate::{dyn_value, test_only_make_eval_value};

    #[test]
    fn test_version_comparison_equal() {
        let left = dyn_value!("1.2.3");
        let right = test_only_make_eval_value!("1.2.3");

        let result = compare_versions(&left, &right, "version_eq");
        assert!(result);
    }

    #[test]
    fn test_version_comparison_greater_than() {
        let left = dyn_value!("1.2.4");
        let right = test_only_make_eval_value!("1.2.3");

        let result = compare_versions(&left, &right, "version_gt");
        assert!(result);
    }

    #[test]
    fn test_version_comparison_less_than() {
        let left = dyn_value!("1.2.3");
        let right = test_only_make_eval_value!("1.2.4");

        let result = compare_versions(&left, &right, "version_lt");
        assert!(result);
    }
}
