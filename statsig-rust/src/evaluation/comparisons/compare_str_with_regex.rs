use crate::{evaluation::evaluator_value::EvaluatorValue, unwrap_or_return, DynamicValue};

pub(crate) fn compare_str_with_regex(value: &DynamicValue, regex_value: &EvaluatorValue) -> bool {
    let value_str = unwrap_or_return!(&value.string_value, false);
    let regex = unwrap_or_return!(&regex_value.regex_value, false);
    regex.is_match(&value_str.value)
}

#[cfg(test)]
mod tests {
    use crate::evaluation::comparisons::compare_str_with_regex;
    use crate::{dyn_value, test_only_make_eval_value};

    #[test]
    fn test_compare_regex_simple() {
        let left = dyn_value!("apple banana pear");
        let mut right = test_only_make_eval_value!("banana");
        right.compile_regex();

        assert!(compare_str_with_regex(&left, &right));
    }

    #[test]
    fn test_compare_regex_complex() {
        let left =
            dyn_value!(r#"{ "name": "Statsig", "version": "4.8.1-beta.32", "license": "ISC" }"#);
        let mut right = test_only_make_eval_value!(r#"version":\s*"4\.8\.\d+"#); // Major.Minor == 4.8
        right.compile_regex();

        assert!(compare_str_with_regex(&left, &right));
    }
}
