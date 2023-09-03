use crate::interpreter::error::InterpreterError;
use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use regex::Regex;

impl<'a> FunctionInitState<'a> {
    pub fn process_template_str(&mut self, template: &str) -> Result<VarPointer, InterpreterError> {
        println!("template str: {}", template);

        todo!()
    }
}

#[derive(Debug, Eq, PartialEq)]
enum InnerTemplatePart<'a> {
    Literal(&'a str),
    Insert(&'a str),
}

lazy_static::lazy_static! {
    static ref INSERT: Regex = Regex::new("(\\$+)(\\{\\s*([^{}]+)\\s*\\})").unwrap();
}

fn match_inserts(template: &str) -> Result<Vec<InnerTemplatePart>, ()> {
    let mut items = Vec::new();

    let mut captures = INSERT.capture_locations();

    let mut re_start = 0;

    while let Some(_) = INSERT.captures_read_at(&mut captures, template, re_start) {
        let (match_start, match_end) = captures.get(0).unwrap();

        if match_start > re_start {
            items.push(InnerTemplatePart::Literal(&template[re_start..match_start]));
        }

        re_start = match_end;

        let (dollars_start, dollars_end) = captures.get(1).unwrap();
        let dollars_count = dollars_end - dollars_start;

        if dollars_count % 2 == 0 {
            let (_, end) = captures.get(2).unwrap();

            items.push(InnerTemplatePart::Literal(
                &template[(dollars_end - dollars_count / 2)..end],
            ));
            continue;
        }

        if dollars_count > 1 {
            items.push(InnerTemplatePart::Literal(
                &template[(dollars_end - (dollars_count - 1) / 2)..dollars_end],
            ));
        }

        let (start, end) = captures.get(3).unwrap();

        items.push(InnerTemplatePart::Insert(&template[start..end]));
    }

    if re_start < template.len() {
        items.push(InnerTemplatePart::Literal(&template[re_start..]));
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use crate::interpreter::expression::template_str::{match_inserts, InnerTemplatePart};

    #[test]
    fn match_inserts_test() {
        let input =
            r#"start ${simple} middle $$$${fake} $${{FAKE}} $$${(some s-exprs (here 42))} end"#;

        let result = match_inserts(input).expect("matching");

        assert_eq!(
            result,
            vec![
                InnerTemplatePart::Literal("start ",),
                InnerTemplatePart::Insert("simple",),
                InnerTemplatePart::Literal(" middle ",),
                InnerTemplatePart::Literal("$${fake}",),
                InnerTemplatePart::Literal(" $${{FAKE}} ",),
                InnerTemplatePart::Literal("$",),
                InnerTemplatePart::Insert("(some s-exprs (here 42))",),
                InnerTemplatePart::Literal(" end",),
            ]
        )
    }
}
