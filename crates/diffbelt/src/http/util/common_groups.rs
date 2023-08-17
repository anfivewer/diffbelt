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

pub struct IdWithNameGroup {
    pub id: String,
    pub name: Box<str>,
}

pub fn id_with_name_group(captures: Captures<'_>) -> IdWithNameGroup {
    let id = captures.name("id").unwrap().as_str();
    let id = match urlencoding::decode(id) {
        Ok(id) => id.to_string(),
        Err(_) => id.to_string(),
    };

    let name = captures.name("name").unwrap().as_str();
    let name = match urlencoding::decode(name) {
        Ok(name) => Box::from(name),
        Err(_) => Box::from(name),
    };

    IdWithNameGroup { id, name }
}
