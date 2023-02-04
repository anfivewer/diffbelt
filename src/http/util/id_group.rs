use regex::Captures;

pub struct IdOnlyGroup(pub String);

pub fn id_only_group(captures: Captures<'_>) -> IdOnlyGroup {
    IdOnlyGroup(captures.name("id").unwrap().as_str().to_string())
}
