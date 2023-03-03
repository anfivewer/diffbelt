use proc_macro::TokenTree;

pub fn remove_first_async<I: Iterator<Item = TokenTree>>(
    tokens: &mut I,
    result_tokens: &mut Vec<TokenTree>,
) {
    for token in tokens {
        match &token {
            TokenTree::Ident(ident) => {
                if ident.to_string() == "async" {
                    return;
                }
            }
            _ => {}
        }

        result_tokens.push(token);
    }
}

pub fn find_fn_arrow<I: Iterator<Item = TokenTree>>(
    tokens: &mut I,
    result_tokens: &mut Vec<TokenTree>,
) {
    let mut prev_is_dash = false;

    for token in tokens {
        match &token {
            TokenTree::Punct(punct) => {
                let c = punct.as_char();

                if prev_is_dash {
                    if c == '>' {
                        result_tokens.push(token);
                        return;
                    }
                }

                prev_is_dash = c == '-';
            }
            _ => {}
        }

        result_tokens.push(token);
    }
}

pub fn collect_except_last<I: Iterator<Item = TokenTree>>(
    tokens: &mut I,
) -> Option<(Vec<TokenTree>, TokenTree)> {
    let mut trees = Vec::new();
    let Some(mut last) = tokens.next() else {
        return None;
    };

    for token in tokens {
        trees.push(last);
        last = token;
    }

    Some((trees, last))
}
