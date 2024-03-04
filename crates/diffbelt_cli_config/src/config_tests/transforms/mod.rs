mod aggregate_map;
pub mod map_filter;

#[macro_export]
macro_rules! call_human_readable_conversion {
    ($value:ident, $human_readable:ident, $method:ident, $input_vec_holder:ident, $output_vec_holder:ident) => {{
        () = $input_vec_holder.replace_with_slice($value.as_bytes())?;
        let slice = $human_readable
            .instance
            .vec_to_bytes_slice(&$input_vec_holder)?;

        () = $human_readable.$method(&slice.0, &$output_vec_holder)?;

        $output_vec_holder.access()?
    }};
}
