use proc_macro::{Delimiter, Group, Ident, Punct, Spacing, Span, TokenStream, TokenTree};
use std::iter::once;

pub fn wrap_with_pin_box_future<I: Iterator<Item = TokenTree>>(
    output: I,
) -> impl Iterator<Item = TokenTree> {
    let pin_stream: TokenStream = "::std::pin::Pin".parse().unwrap();
    let box_stream: TokenStream = "::std::boxed::Box".parse().unwrap();
    let future_stream: TokenStream = "::std::future::Future".parse().unwrap();
    let send_stream: TokenStream = "::std::marker::Send".parse().unwrap();

    pin_stream
        .into_iter()
        .chain(once(TokenTree::Punct(Punct::new('<', Spacing::Alone))))
        .chain(box_stream.into_iter())
        .chain(
            [
                TokenTree::Punct(Punct::new('<', Spacing::Alone)),
                TokenTree::Ident(Ident::new("dyn", Span::call_site())),
            ]
            .into_iter(),
        )
        .chain(future_stream.into_iter())
        .chain(
            [
                TokenTree::Punct(Punct::new('<', Spacing::Alone)),
                TokenTree::Ident(Ident::new("Output", Span::call_site())),
                TokenTree::Punct(Punct::new('=', Spacing::Alone)),
            ]
            .into_iter(),
        )
        .chain(output)
        .chain(
            [
                TokenTree::Punct(Punct::new('>', Spacing::Alone)),
                TokenTree::Punct(Punct::new('+', Spacing::Alone)),
            ]
            .into_iter(),
        )
        .chain(send_stream.into_iter())
        .chain(
            [
                TokenTree::Punct(Punct::new('+', Spacing::Alone)),
                TokenTree::Punct(Punct::new('\'', Spacing::Joint)),
                TokenTree::Ident(Ident::new("static", Span::call_site())),
                TokenTree::Punct(Punct::new('>', Spacing::Alone)),
                TokenTree::Punct(Punct::new('>', Spacing::Alone)),
            ]
            .into_iter(),
        )
}

pub fn wrap_with_box_pin_async(body: TokenTree) -> impl Iterator<Item = TokenTree> {
    let box_pin_tokens: TokenStream = "::std::boxed::Box::pin".parse().unwrap();
    let async_move_tokens: TokenStream = "async move".parse().unwrap();

    box_pin_tokens
        .into_iter()
        .chain(once(TokenTree::Group(Group::new(
            Delimiter::Parenthesis,
            async_move_tokens.into_iter().chain(once(body)).collect(),
        ))))
}
