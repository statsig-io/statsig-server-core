use super::dynamic_string::DynamicString;
use crate::evaluation::evaluator_context::EvaluatorContext;
use crate::DynamicValue;
use lazy_static::lazy_static;

lazy_static! {
    static ref EMPTY_STR: String = String::new();
    static ref EMPTY_DYNAMIC_VALUE: DynamicValue = DynamicValue::new();
}

pub(crate) fn get_unit_id<'a>(
    ctx: &'a mut EvaluatorContext,
    id_type: &'a DynamicString,
) -> &'a String {
    ctx.user
        .get_unit_id(id_type)
        .unwrap_or(&EMPTY_DYNAMIC_VALUE)
        .string_value
        .as_ref()
        .map(|s| &s.value)
        .unwrap_or(&EMPTY_STR)
}
