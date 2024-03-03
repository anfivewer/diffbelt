mod aggregate_map;
pub mod map_filter;

#[macro_export]
macro_rules! call_human_readable_conversion {
    ($value:ident, $human_readable:ident, $method:ident, $input_vec_holder:ident, $output_vec_holder:ident) => {{
        () = $human_readable
            .instance
            .replace_vec_with_slice(&$input_vec_holder, $value.as_bytes())?;
        let slice = $human_readable
            .instance
            .vec_to_bytes_slice(&$input_vec_holder)?;

        () = $human_readable.$method(&slice.0, &$output_vec_holder)?;

        $output_vec_holder.access()?
    }};
}
