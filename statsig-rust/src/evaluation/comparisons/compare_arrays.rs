use crate::{evaluation::evaluator_value::MemoizedEvaluatorValue, unwrap_or_return, DynamicValue};
use std::collections::HashSet;

pub(crate) fn compare_arrays(
    value: &DynamicValue,
    target_value: &MemoizedEvaluatorValue,
    op: &str,
) -> bool {
    let target_array = unwrap_or_return!(&target_value.array_value, false);
    let value_array = unwrap_or_return!(&value.array_value, false);
    let empty_string = String::new();
    let value_set: HashSet<&String> = HashSet::from_iter(value_array.iter().map(|x| {
        x.string_value
            .as_ref()
            .map(|s| &s.value)
            .unwrap_or(&empty_string)
    }));

    for (_, item) in target_array.values() {
        match op {
            "array_contains_all" => {
                if !value_set.contains(&item) {
                    return false;
                }
            }
            "array_contains_any" => {
                if value_set.contains(&item) {
                    return true;
                }
            }
            "array_contains_none" => {
                if value_set.contains(&item) {
                    return false;
                }
            }
            "not_array_contains_all" => {
                if !value_set.contains(&item) {
                    return true;
                }
            }

            _ => {
                return false;
            }
        }
    }
    !(op == "array_contains_any" || op == "not_array_contains_all")
}
