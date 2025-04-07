use crate::{unwrap_or_return, DynamicValue};

pub(crate) fn compare_str_with_regex(value: &DynamicValue, regex_value: &DynamicValue) -> bool {
    let value_str = unwrap_or_return!(&value.string_value, false);
    let regex = unwrap_or_return!(&regex_value.regex_value, false);
    regex.is_match(value_str)
}

#[cfg(test)]
mod tests {
    use crate::dyn_value;
    use crate::evaluation::comparisons::compare_str_with_regex;

    #[test]
    fn test_compare_regex_simple() {
        let left = dyn_value!("apple banana pear");
        let mut right = dyn_value!("banana");
        right.compile_regex();

        assert!(compare_str_with_regex(&left, &right));
    }

    #[test]
    fn test_compare_regex_complex() {
        let left =
            dyn_value!(r#"{ "name": "Statsig", "version": "4.8.1-beta.32", "license": "ISC" }"#);
        let mut right = dyn_value!(r#"version":\s*"4\.8\.\d+"#); // Major.Minor == 4.8
        right.compile_regex();

        assert!(compare_str_with_regex(&left, &right));
    }
}
