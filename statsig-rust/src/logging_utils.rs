/// Sanitizes SDK keys embedded in strings by masking characters after `secret-`.
/// Keeps the first 5 chars of the key and replaces the rest with `*****`.
/// Use this for ANY log output that may include URLs or messages containing SDK keys.
pub fn sanitize_secret_key(input: &str) -> String {
    input
        .split("secret-")
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                part.to_string()
            } else {
                let (key, rest) =
                    part.split_at(part.chars().take_while(|c| c.is_alphanumeric()).count());
                let sanitized_key = if key.len() > 5 {
                    format!("{}*****{}", &key[..5], rest)
                } else {
                    format!("{key}*****{rest}")
                };
                format!("secret-{sanitized_key}")
            }
        })
        .collect()
}
