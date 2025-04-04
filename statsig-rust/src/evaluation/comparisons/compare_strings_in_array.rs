use crate::{unwrap_or_return, DynamicValue};

pub(crate) fn compare_strings_in_array(
    value: &DynamicValue,
    target_value: &DynamicValue,
    op: &str,
    ignore_case: bool,
) -> bool {
    let empty_str = String::new();
    let value_str = value.string_value.as_ref().unwrap_or(&empty_str);
    let lowered_value_str = value.lowercase_string_value.as_ref().unwrap_or(&empty_str);

    let result = {
        if op == "any" || op == "none" {
            if let Some(dict) = &target_value.object_value {
                let contains = dict.contains_key(value_str);
                return if op == "none" { !contains } else { contains };
            }
        }

        let array = unwrap_or_return!(&target_value.array_value, false);

        array.iter().any(|current| {
            let (curr_str, curr_lower_str) =
                match (&current.string_value, &current.lowercase_string_value) {
                    (Some(s), Some(ls)) => (s, ls),
                    _ => return false,
                };
            let left = if ignore_case {
                lowered_value_str
            } else {
                value_str
            };
            let right = if ignore_case {
                curr_lower_str
            } else {
                curr_str
            };

            match op {
                "any" | "none" | "any_case_sensitive" | "none_case_sensitive" => left.eq(right),
                "str_starts_with_any" => left.starts_with(right),
                "str_ends_with_any" => left.ends_with(right),
                "str_contains_any" | "str_contains_none" => left.contains(right),
                _ => false,
            }
        })
    };

    if op == "none" || op == "none_case_sensitive" || op == "str_contains_none" {
        return !result;
    }
    result
}

#[cfg(test)]
mod tests {
    use crate::dyn_value;
    use crate::evaluation::comparisons::compare_strings_in_array;
    use serde_json::json;

    #[test]
    fn test_array_contains() {
        let needle = dyn_value!("Foo");
        let haystack = dyn_value!(json!(vec!["boo", "bar", "foo", "far", "zoo", "zar"]));

        assert!(compare_strings_in_array(&needle, &haystack, "any", true));

        assert!(!compare_strings_in_array(&needle, &haystack, "any", false));
    }

    #[test]
    fn test_array_does_not_contain() {
        let needle = dyn_value!("Foo");
        let haystack = dyn_value!(json!(vec!["boo", "bar", "far", "zoo", "zar"]));

        assert!(!compare_strings_in_array(&needle, &haystack, "any", true));

        assert!(!compare_strings_in_array(&needle, &haystack, "any", false));
    }

    #[test]
    fn test_str_starting_with() {
        let needle = dyn_value!("daniel@statsig.com");
        let haystack = dyn_value!(json!(vec!["tore", "daniel"]));

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
        let haystack = dyn_value!(json!(vec!["@statsig.io", "@statsig.com"]));

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
        let haystack = dyn_value!(json!(vec!["sigstat", "statsig"]));

        assert!(compare_strings_in_array(
            &needle,
            &haystack,
            "str_contains_any",
            true
        ));
    }
}
