use std::ptr;

use unsafe_libyaml::{
    yaml_emitter_emit, yaml_mapping_end_event_initialize, yaml_mapping_start_event_initialize,
    yaml_mapping_style_t,
};

use crate::serialization::{check_error, SerializationContext};
use crate::{YamlMapping, YamlSerializationError};

pub(crate) unsafe fn emit_mapping(
    ctx: &mut SerializationContext,
    mapping: &YamlMapping,
) -> Result<(), YamlSerializationError> {
    let event = ctx.take_event();

    let result = yaml_mapping_start_event_initialize(
        event,
        ptr::null(),
        ptr::null(),
        false,
        yaml_mapping_style_t::YAML_BLOCK_MAPPING_STYLE,
    );
    let () = check_error(result.ok, ctx.emitter)?;

    let result = yaml_emitter_emit(ctx.emitter, event);
    let () = check_error(result.ok, ctx.emitter)?;

    for (key, value) in &mapping.items {
        let () = key.serialize_node(ctx)?;
        let () = value.serialize_node(ctx)?;
    }

    let event = ctx.take_event();

    let result = yaml_mapping_end_event_initialize(event);
    let () = check_error(result.ok, ctx.emitter)?;

    let result = yaml_emitter_emit(ctx.emitter, event);
    let () = check_error(result.ok, ctx.emitter)?;

    Ok(())
}
