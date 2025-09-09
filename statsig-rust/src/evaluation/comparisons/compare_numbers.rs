use crate::{evaluation::evaluator_value::MemoizedEvaluatorValue, unwrap_or_return, DynamicValue};

pub(crate) fn compare_numbers(
    left: &DynamicValue,
    right: &MemoizedEvaluatorValue,
    op: &str,
) -> bool {
    let left_num = unwrap_or_return!(left.float_value, false);
    let right_num = unwrap_or_return!(right.float_value, false);

    match op {
        "gt" => left_num > right_num,
        "gte" => left_num >= right_num,
        "lt" => left_num < right_num,
        "lte" => left_num <= right_num,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::evaluation::comparisons::compare_numbers;
    use crate::{dyn_value, test_only_make_eval_value};

    #[test]
    fn test_number_greater_than() {
        let left = dyn_value!(2.0);
        let right = test_only_make_eval_value!(1.0);

        let result = compare_numbers(&left, &right, "gt");
        assert!(result);
    }

    #[test]
    fn test_number_greater_than_equal_string() {
        let left = dyn_value!("1.24");
        let right_smaller = test_only_make_eval_value!("1.23");
        let right_same = test_only_make_eval_value!("1.24");

        assert!(compare_numbers(&left, &right_smaller, "gte"));
        assert!(compare_numbers(&left, &right_same, "gte"));
    }

    #[test]
    fn test_number_less_than_equal_string() {
        let left = dyn_value!("1.23");
        let right_bigger = test_only_make_eval_value!("1.24");
        let right_same = test_only_make_eval_value!("1.24");

        assert!(compare_numbers(&left, &right_bigger, "lte"));
        assert!(compare_numbers(&left, &right_same, "lte"));
    }

    #[test]
    fn test_number_less_than() {
        let left = dyn_value!(1.0);
        let right = test_only_make_eval_value!(2.0);

        let result = compare_numbers(&left, &right, "lt");
        assert!(result);
    }

    #[test]
    fn test_number_less_than_or_equal() {
        let dyn_one = dyn_value!(1.0);
        let dyn_two = dyn_value!(2.0);
        let eval_one = test_only_make_eval_value!(1.0);
        let eval_two = test_only_make_eval_value!(2.0);

        assert!(compare_numbers(&dyn_one, &eval_two, "lte"));
        assert!(compare_numbers(&dyn_two, &eval_two, "lte"));
        assert!(!compare_numbers(&dyn_two, &eval_one, "lte"));
    }
}
