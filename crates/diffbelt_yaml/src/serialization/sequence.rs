use std::ptr;

use unsafe_libyaml::{
    yaml_emitter_emit, yaml_sequence_end_event_initialize, yaml_sequence_start_event_initialize,
    yaml_sequence_style_t,
};

use crate::serialization::{check_error, SerializationContext};
use crate::{YamlSequence, YamlSerializationError};

pub(crate) unsafe fn emit_sequence(
    ctx: &mut SerializationContext,
    sequence: &YamlSequence,
) -> Result<(), YamlSerializationError> {
    let event = ctx.take_event();

    let result = yaml_sequence_start_event_initialize(
        event,
        ptr::null(),
        ptr::null(),
        false,
        yaml_sequence_style_t::YAML_BLOCK_SEQUENCE_STYLE,
    );
    () = check_error(result.ok, ctx.emitter)?;

    let result = yaml_emitter_emit(ctx.emitter, event);
    () = check_error(result.ok, ctx.emitter)?;

    for node in &sequence.items {
        () = node.serialize_node(ctx)?;
    }

    let event = ctx.take_event();

    let result = yaml_sequence_end_event_initialize(event);
    () = check_error(result.ok, ctx.emitter)?;

    let result = yaml_emitter_emit(ctx.emitter, event);
    () = check_error(result.ok, ctx.emitter)?;

    Ok(())
}
