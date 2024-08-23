use crate::{unwrap_or_return, DynamicValue};

pub(crate) fn compare_numbers(left: &DynamicValue, right: &DynamicValue, op: &str) -> bool {
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
    use crate::{dyn_value, DynamicValue};
    use crate::evaluation::comparisons::compare_numbers;

    #[test]
    fn test_number_greater_than() {
        let left = dyn_value!(2);
        let right = dyn_value!(1);

        let result = compare_numbers(&left, &right, "gt");
        assert!(result)
    }

    #[test]
    fn test_number_greater_than_equal_string() {
        let left = dyn_value!("1.24");
        let right = dyn_value!("1.23");

        assert!(compare_numbers(&left, &right, "gte"));
        assert!(compare_numbers(&left, &left, "gte"));
    }

    #[test]
    fn test_number_less_than_equal_string() {
        let left = dyn_value!("1.23");
        let right = dyn_value!("1.24");

        assert!(compare_numbers(&left, &right, "lte"));
        assert!(compare_numbers(&left, &left, "lte"));
    }

    #[test]
    fn test_number_less_than() {
        let left = dyn_value!(1);
        let right = dyn_value!(2);

        let result = compare_numbers(&left, &right, "lt");
        assert!(result)
    }

    #[test]
    fn test_number_less_than_or_equal() {
        let one = dyn_value!(1);
        let two = dyn_value!(2);

        assert!(compare_numbers(&one, &two, "lte"));
        assert!(compare_numbers(&two, &two, "lte"));
        assert!(!compare_numbers(&two, &one, "lte"));
    }
}
