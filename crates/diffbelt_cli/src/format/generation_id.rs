use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;

pub fn format_generation_id(generation_id: &EncodedGenerationIdJsonData) -> String {
    let EncodedGenerationIdJsonData { value, encoding } = generation_id;

    let encoding = encoding.as_ref().map(|x| x.as_str()).unwrap_or("utf8");

    if encoding == "utf8" {
        return format!("\"{}\"", value.replace("\"", "\\\""));
    }

    if encoding == "base64" {
        return format!("base64:{}", value);
    }

    panic!("unknown generationId encoding: {}", encoding);
}
