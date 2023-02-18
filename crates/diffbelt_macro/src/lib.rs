use crate::pin_box_future::{wrap_with_box_pin_async, wrap_with_pin_box_future};
use crate::tokens_util::{collect_except_last, find_fn_arrow, remove_first_async};
use proc_macro::{Delimiter, Group, TokenStream, TokenTree};
use std::iter::once;

mod pin_box_future;
mod tokens_util;

#[proc_macro_attribute]
pub fn fn_box_pin_async(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut tokens_before_return_type = Vec::new();

    let mut iter = item.into_iter();

    remove_first_async(&mut iter, &mut tokens_before_return_type);
    find_fn_arrow(&mut iter, &mut tokens_before_return_type);

    let (return_type, body) = collect_except_last(&mut iter).unwrap();

    let new_return_type = wrap_with_pin_box_future(return_type.into_iter());
    let new_body = wrap_with_box_pin_async(body);

    let result: TokenStream = tokens_before_return_type
        .into_iter()
        .chain(new_return_type)
        .chain(once(TokenTree::Group(Group::new(
            Delimiter::Brace,
            new_body.collect(),
        ))))
        .collect();

    result
}
