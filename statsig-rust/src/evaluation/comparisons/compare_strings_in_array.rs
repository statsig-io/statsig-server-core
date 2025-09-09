use crate::{
    evaluation::{dynamic_string::DynamicString, evaluator_value::MemoizedEvaluatorValue},
    unwrap_or_return, DynamicValue,
};

pub(crate) fn compare_strings_in_array(
    value: &DynamicValue,
    target_value: &MemoizedEvaluatorValue,
    op: &str,
    ignore_case: bool,
) -> bool {
    let empty_str = DynamicString::from(String::new());
    let value_str = value.string_value.as_ref().unwrap_or(&empty_str);

    let result = {
        if let Some(keyed_lookup) = &target_value.object_value {
            if ignore_case && (op == "any" || op == "none") {
                let contains = keyed_lookup.contains_key(&value_str.lowercased_value);
                return if op == "none" { !contains } else { contains };
            }
        }

        let array_value = unwrap_or_return!(&target_value.array_value, false);
        if ignore_case && (op == "any" || op == "none") {
            let contains = array_value.contains_key(&value_str.lowercased_value);
            return if op == "none" { !contains } else { contains };
        }

        let mut comparison_result = false;
        for (lowercase_str, (_, current_str)) in array_value {
            let left = if ignore_case {
                &value_str.lowercased_value
            } else {
                &value_str.value
            };

            let right = if ignore_case {
                lowercase_str
            } else {
                current_str
            };

            comparison_result = match op {
                "any" | "none" | "any_case_sensitive" | "none_case_sensitive" => left.eq(right),
                "str_starts_with_any" => left.starts_with(right),
                "str_ends_with_any" => left.ends_with(right),
                "str_contains_any" | "str_contains_none" => left.contains(right),
                _ => false, // todo: unsupported?
            };

            if comparison_result {
                break;
            }
        }

        comparison_result
    };

    if op == "none" || op == "none_case_sensitive" || op == "str_contains_none" {
        return !result;
    }
    result
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::evaluation::comparisons::compare_strings_in_array;
    use crate::{dyn_value, test_only_make_eval_value};

    #[test]
    fn test_array_contains() {
        let needle = dyn_value!("Foo");
        let haystack = test_only_make_eval_value!(["boo", "bar", "foo", "far", "zoo", "zar"]);

        assert!(compare_strings_in_array(&needle, &haystack, "any", true));
        assert!(!compare_strings_in_array(&needle, &haystack, "any", false));
    }

    #[test]
    fn test_array_does_not_contain() {
        let needle = dyn_value!("Foo");
        let haystack = test_only_make_eval_value!(vec!["boo", "bar", "far", "zoo", "zar"]);

        assert!(!compare_strings_in_array(&needle, &haystack, "any", true));

        assert!(!compare_strings_in_array(&needle, &haystack, "any", false));
    }

    #[test]
    fn test_str_starting_with() {
        let needle = dyn_value!("daniel@statsig.com");
        let haystack = test_only_make_eval_value!(vec!["tore", "daniel"]);

        assert!(compare_strings_in_array(
            &needle,
            &haystack,
            "str_starts_with_any",
            true
        ));
    }

    #[test]
    fn test_str_ending_with() {
        let needle = dyn_value!("tore@statsig.com");
        let haystack = test_only_make_eval_value!(vec!["@statsig.io", "@statsig.com"]);

        assert!(compare_strings_in_array(
            &needle,
            &haystack,
            "str_ends_with_any",
            true
        ));
    }

    #[test]
    fn test_str_contains_any() {
        let needle = dyn_value!("daniel@statsig.io");
        let haystack = test_only_make_eval_value!(vec!["sigstat", "statsig"]);

        assert!(compare_strings_in_array(
            &needle,
            &haystack,
            "str_contains_any",
            true
        ));
    }

    #[test]
    fn test_str_none_case_sensitive() {
        let haystack = test_only_make_eval_value!(vec!["HELLO", "WORLD"]);

        let upper_needle = dyn_value!("HELLO");
        assert!(!compare_strings_in_array(
            &upper_needle,
            &haystack,
            "none_case_sensitive",
            false
        ));

        let lower_needle = dyn_value!("hello");
        assert!(compare_strings_in_array(
            &lower_needle,
            &haystack,
            "none_case_sensitive",
            false
        ));
    }

    #[test]
    fn test_str_any_case_sensitive() {
        let haystack = test_only_make_eval_value!(vec!["HELLO", "WORLD"]);

        let upper_needle = dyn_value!("HELLO");
        assert!(compare_strings_in_array(
            &upper_needle,
            &haystack,
            "any_case_sensitive",
            false
        ));

        let lower_needle = dyn_value!("hello");
        assert!(!compare_strings_in_array(
            &lower_needle,
            &haystack,
            "any_case_sensitive",
            false
        ));
    }

    #[test]
    fn test_array_contains_any() {
        let needle = dyn_value!(json!(["boo", 1, true]));
        let haystack_positive = test_only_make_eval_value!(vec!["zoo", "boo"]);
        let haystack_negative = test_only_make_eval_value!(vec!["zoo", "bar"]);

        assert!(compare_strings_in_array(
            &needle,
            &haystack_positive,
            "str_contains_any",
            true
        ));

        assert!(!compare_strings_in_array(
            &needle,
            &haystack_negative,
            "str_contains_any",
            true
        ));
    }
}
