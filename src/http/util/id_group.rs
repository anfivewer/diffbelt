use regex::Captures;

pub struct IdOnlyGroup(pub String);

pub fn id_only_group(captures: Captures<'_>) -> IdOnlyGroup {
    let id = captures.name("id").unwrap().as_str();
    let id = match urlencoding::decode(id) {
        Ok(id) => id.to_string(),
        Err(_) => id.to_string(),
    };

    IdOnlyGroup(id)
}
