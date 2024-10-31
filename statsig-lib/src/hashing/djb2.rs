pub fn djb2(input: &String) -> String {
    let mut hash: i64 = 0;

    for c in input.chars() {
        hash = ((hash << 5) - hash) + c as i64;
        hash &= hash;
    }

    // Convert to unsigned 32-bit integer
    hash &= 0xFFFFFFFF;

    hash.to_string()
}
