#[must_use]
pub fn djb2(input: &str) -> String {
    let mut hash: i64 = 0;

    for c in input.chars() {
        hash = ((hash << 5).wrapping_sub(hash)).wrapping_add(c as i64);
    }

    // Convert to unsigned 32-bit integer
    hash &= 0xFFFF_FFFF;

    hash.to_string()
}
