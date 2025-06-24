use super::super::tokenizer::{Token, Tokenizer};

#[test]
fn windows_user_agent() {
    let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 Trailer/93.3.8652.5";
    let result = Tokenizer::run(user_agent);

    let possible_os = flatten_opt_token(&result.possible_os_token);

    assert!(result.windows_hint);
    assert_eq!(possible_os, Some(("Windows", Some("10.0"))));

    let tokens = result.tokens;
    print_tokens(&tokens);

    assert_eq!(flatten_token(&tokens[0]), ("Mozilla", Some("5.0")));
    assert_eq!(flatten_token(&tokens[1]), ("Windows", Some("10.0")));
    assert_eq!(flatten_token(&tokens[2]), ("Win64", None));
    assert_eq!(flatten_token(&tokens[3]), ("x64", None));
    assert_eq!(flatten_token(&tokens[4]), ("AppleWebKit", Some("537.36")));
    assert_eq!(flatten_token(&tokens[7]), ("Chrome", Some("134.0.0.0")));
    assert_eq!(flatten_token(&tokens[8]), ("Safari", Some("537.36")));
    assert_eq!(flatten_token(&tokens[9]), ("Trailer", Some("93.3.8652.5")));
}

fn flatten_opt_token<'a>(token: &Option<Token<'a>>) -> Option<(&'a str, Option<&'a str>)> {
    token.as_ref().map(|t| (t.tag, t.version))
}

fn flatten_token<'a>(token: &Token<'a>) -> (&'a str, Option<&'a str>) {
    (token.tag, token.version)
}

fn print_tokens<'a>(tokens: &[Token<'a>]) {
    for token in tokens {
        println!("[{}] [{}]", token.tag, token.version.unwrap_or_default());
    }
}
