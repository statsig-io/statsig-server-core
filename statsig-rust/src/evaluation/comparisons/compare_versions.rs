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

    let result = match compare_versions_impl(left_str, right_str) {
        ComparisonResult::Ok(result) => result,
        ComparisonResult::NumericParseFailure => return false,
    };

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

enum ComparisonResult {
    NumericParseFailure,
    Ok(i32),
}

fn compare_versions_impl(left_str: &str, right_str: &str) -> ComparisonResult {
    let left_version = left_str.split('-').next().unwrap_or("");
    let right_version = right_str.split('-').next().unwrap_or("");

    let mut left_parts = left_version.split('.');
    let mut right_parts = right_version.split('.');

    loop {
        let left_num = match next_num(left_parts.next()) {
            Ok(v) => v,
            Err(_) => return ComparisonResult::NumericParseFailure,
        };

        let right_num = match next_num(right_parts.next()) {
            Ok(v) => v,
            Err(_) => return ComparisonResult::NumericParseFailure,
        };

        if left_num.is_none() && right_num.is_none() {
            break;
        }

        let left_num = left_num.unwrap_or(0);
        let right_num = right_num.unwrap_or(0);

        if left_num < right_num {
            return ComparisonResult::Ok(-1);
        }

        if left_num > right_num {
            return ComparisonResult::Ok(1);
        }
    }

    ComparisonResult::Ok(0)
}

fn next_num(part: Option<&str>) -> Result<Option<i128>, std::num::ParseIntError> {
    part.map(|s| s.trim().parse::<i128>()).transpose()
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
