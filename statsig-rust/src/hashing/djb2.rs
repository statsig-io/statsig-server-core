#[must_use]
pub fn djb2(input: &str) -> String {
    djb2_number(input).to_string()
}

#[must_use]
pub fn djb2_number(input: &str) -> i64 {
    let mut hash = 0_i32;

    for character in input.encode_utf16() {
        hash = hash
            .wrapping_shl(5)
            .wrapping_sub(hash)
            .wrapping_add(i32::from(character));
    }

    i64::from(hash as u32)
}
