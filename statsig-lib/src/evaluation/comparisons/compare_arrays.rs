use crate::{unwrap_or_return, DynamicValue};
use std::collections::HashSet;

pub(crate) fn compare_arrays(
    value: &DynamicValue,
    target_value: &DynamicValue,
    op: &str,
) -> bool {
    let target_array = unwrap_or_return!(&target_value.array_value, false);
    let value_array = unwrap_or_return!(&value.array_value, false);
    let value_set: HashSet<Option<String>> = HashSet::from_iter(value_array.iter().map(|x| x.string_value.clone()));
    for item in target_array.iter() {
        match op {
            "array_contains_all" => {
                if !value_set.contains(&item.string_value) {
                    return false;
                }
            }
            "array_contains_any" => {
                if value_set.contains(&item.string_value) {
                    return true;
                }
            }
            "array_contains_none" => {
                if value_set.contains(&item.string_value) {
                    return false;
                }
            }
            "not_array_contains_all" => {
                if !value_set.contains(&item.string_value) {
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