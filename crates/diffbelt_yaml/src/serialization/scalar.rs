use std::ptr;

use unsafe_libyaml::{yaml_emitter_emit, yaml_scalar_event_initialize, yaml_scalar_style_t};

use diffbelt_util_no_std::cast::checked_usize_to_i32;

use crate::serialization::{check_error, SerializationContext};
use crate::{YamlScalar, YamlSerializationError};

pub(crate) unsafe fn emit_scalar(
    ctx: &mut SerializationContext,
    scalar: &YamlScalar,
) -> Result<(), YamlSerializationError> {
    let event = ctx.take_event();

    let contains_newlines = scalar.value.contains('\n');

    let style = if contains_newlines {
        yaml_scalar_style_t::YAML_FOLDED_SCALAR_STYLE
    } else {
        let first_char = scalar.value.chars().next();

        match first_char {
            Some('\'') => yaml_scalar_style_t::YAML_DOUBLE_QUOTED_SCALAR_STYLE,
            Some('"') => yaml_scalar_style_t::YAML_SINGLE_QUOTED_SCALAR_STYLE,
            Some(_) | None => yaml_scalar_style_t::YAML_ANY_SCALAR_STYLE,
        }
    };

    let result = yaml_scalar_event_initialize(
        event,
        ptr::null(),
        ptr::null(),
        scalar.value.as_bytes().as_ptr(),
        checked_usize_to_i32(scalar.value.len()),
        true,
        true,
        style,
    );
    let () = check_error(result.ok, ctx.emitter)?;

    let result = yaml_emitter_emit(ctx.emitter, event);
    let () = check_error(result.ok, ctx.emitter)?;

    Ok(())
}
