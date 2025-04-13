mod generated;
pub mod handlers;
pub mod impls;
mod tests;

pub mod api {
    pub mod collection {
        pub use super::super::generated::api::collection_generated::*;
    }
    pub mod common {
        pub use super::super::generated::api::common_generated::*;
    }
    pub mod methods {
        pub use super::super::generated::api::methods_generated::*;
    }
    pub mod phantom {
        pub use super::super::generated::api::phantom_generated::*;
    }
    pub mod get {
        pub use super::super::generated::api::get_generated::*;
    }
    pub mod get_keys_around {
        pub use super::super::generated::api::get_keys_around_generated::*;
    }
    pub mod put_many {
        pub use super::super::generated::api::put_many_generated::*;
    }
    pub mod generation {
        pub use super::super::generated::api::generation_generated::*;
    }
    pub mod query {
        pub use super::super::generated::api::query_generated::*;
    }

    pub mod readers {
        pub use super::super::generated::api::readers_generated::*;
    }
}

pub mod transform {
    pub mod map_filter {
        use core::fmt::{Display, Formatter, Write};

        use diffbelt_util_no_std::fmt::bytes::fmt_bytes_as_str_or_hex;

        pub use super::super::generated::transform::map_filter_generated::*;

        impl Display for MapFilterInput<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                let source_key = self.source_key().map(|x| x.bytes());
                let source_old_value = self.source_old_value().map(|x| x.bytes());
                let source_new_value = self.source_new_value().map(|x| x.bytes());

                let mut need_new_line = false;

                fn write_line(
                    need_new_line: &mut bool,
                    prefix: &'static str,
                    maybe_bytes: Option<&[u8]>,
                    f: &mut Formatter<'_>,
                ) -> core::fmt::Result {
                    let Some(bytes) = maybe_bytes else {
                        return Ok(());
                    };

                    if *need_new_line {
                        f.write_char('\n')?;
                    }

                    f.write_str(prefix)?;
                    fmt_bytes_as_str_or_hex(bytes, f)?;

                    *need_new_line = true;

                    Ok(())
                }

                write_line(&mut need_new_line, "source_key: ", source_key, f)?;
                write_line(
                    &mut need_new_line,
                    "source_old_value: ",
                    source_old_value,
                    f,
                )?;
                write_line(
                    &mut need_new_line,
                    "source_new_value: ",
                    source_new_value,
                    f,
                )?;

                Ok(())
            }
        }
    }

    pub mod aggregate {
        pub use super::super::generated::transform::aggregate_generated::*;
    }
}
