use crate::global::take_buffer_for_realign;
use crate::update_ms::percentiles::constants::PERCENTILES;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::str::from_utf8;
use diffbelt_aligned_bytes::AlignedBytes;
use diffbelt_example_protos::protos::impls::{
    UpdateMsAccumulatorProto, UpdateMsIntermediateDiffProto,
};
use diffbelt_example_protos::protos::update_ms::UpdateMsAccumulator;
use diffbelt_protos::protos::impls::GetKeysAroundResponseProto;
use diffbelt_protos::protos::transform::aggregate::AggregateReduceInput;
use diffbelt_protos::{deserialize, OwnedSerialized};
use diffbelt_util_no_std::cast::{f32_to_f64, f32_to_u32, u64_to_f32};
use diffbelt_wasm_binding::debug_print_string;
use diffbelt_wasm_binding::requests::RequestId;
use hashbrown::HashMap;
use lazy_static::lazy_static;
use libm::roundf;
use regex::Regex;
use parsed_intermediate_key::ParsedIntermediateKey;
use percentile::Percentile;
use crate::update_ms::percentiles::reduce::keys_around::KeysAround;
use crate::update_ms::percentiles::reduce::parsed_intermediate_key::parse_and_store_intermediate_key;

mod percentile;
mod parsed_intermediate_key;
mod keys_around;

struct ByType {
    sum_ms: f64,
    count: u64,
}

pub struct ReduceAccumulator {
    by_type: HashMap<String, ByType>,
    items_count: u64,
    percentiles: [Percentile; PERCENTILES.len()],
}

impl ReduceAccumulator {
    pub fn new(data: UpdateMsAccumulator<'_>) -> Self {
        let data_by_type = data.by_type().unwrap_or_default();
        let data_percentiles = data.percentiles().unwrap_or_default();

        assert_eq!(
            data_percentiles.len(),
            PERCENTILES.len(),
            "percentiles are immutable"
        );

        let mut by_type: HashMap<String, ByType> = HashMap::with_capacity(data_by_type.len());
        let mut items_count = 0;

        for by_type_entry in data_by_type {
            let key = by_type_entry.key().expect("no key");
            let sum_ms = by_type_entry.sum_ms();
            let count = by_type_entry.count();

            items_count += count;

            by_type.insert(String::from(key), ByType { sum_ms, count });
        }

        let mut index = 0usize;
        let percentiles = PERCENTILES.map(|p| {
            let percentile = data_percentiles.get(index);
            let perc = percentile.percentile().expect("no percentile");
            index += 1;

            assert_eq!(perc.percentile(), p, "percentiles are immutable");

            let intermediate_key = perc
                .intermediate_key()
                .map(|x| parsed_intermediate_key::parse_and_store_intermediate_key(x.bytes()));
            let keys_around = percentile.keys_around().map(|x| x.bytes());
            let keys_around = parse_keys_around(keys_around);
            let next_request_id = RequestId(percentile.next_request_id());

            Percentile {
                percentile: p,
                intermediate_key,
                key_pos: percentile.key_pos(),
                keys_around,
                next_request_id,
                next_request_is_forward: percentile.next_request_is_forward(),
            }
        });

        Self {
            by_type,
            items_count,
            percentiles,
        }
    }

    pub fn reduce(
        &mut self,
        input: AggregateReduceInput<'_>,
    ) -> OwnedSerialized<UpdateMsAccumulatorProto> {
        let mut realign_buffer = take_buffer_for_realign();
        for item in input.items().unwrap_or_default() {
            let Some(item) = item.mapped_value() else {
                continue;
            };

            let item = item.bytes();
            let item = AlignedBytes::ensure_alignment_or_copy(item, realign_buffer.as_mut())
                .expect("align");
            let item = deserialize::<UpdateMsIntermediateDiffProto>(item).expect("parse");

            let key = item.key().expect("no key").bytes();
            let old_update_type = item.old_update_type();
            let new_update_type = item.new_update_type();
            let old_ms = item.old_ms();
            let new_ms = item.new_ms();

            let mut need_delete_old_type = false;
            if let Some(old_type) = old_update_type {
                if let Some(by_type) = self.by_type.get_mut(old_type) {
                    by_type.sum_ms -= f32_to_f64(old_ms);
                    by_type.count -= 1;
                    if by_type.count == 0 {
                        need_delete_old_type = true;
                    }
                }
            }
            if let Some(new_type) = new_update_type {
                let by_type = self.by_type.get_mut(new_type);
                let by_type = match by_type {
                    Some(x) => x,
                    None => {
                        self.by_type.insert(
                            String::from(new_type),
                            ByType {
                                sum_ms: 0.0,
                                count: 0,
                            },
                        );
                        self.by_type.get_mut(new_type).expect("just inserted")
                    }
                };

                by_type.sum_ms += f32_to_f64(new_ms);
                by_type.count += 1;
            }

            if need_delete_old_type && old_update_type != new_update_type {
                self.by_type
                    .remove(old_update_type.expect("should be Some"));
            }

            self.process_percentiles(key, old_update_type.is_some(), new_update_type.is_some());
        }

        todo!()
    }

    fn process_percentiles(&mut self, key: &[u8], had_existed: bool, now_existing: bool) {
        if had_existed == now_existing {
            // nothing was changed
            return;
        }

        let key = parse_and_store_intermediate_key(key);
        let value = key.value();

        let new_count = if now_existing {
            self.items_count + 1
        } else {
            self.items_count - 1
        };
        self.items_count = new_count;

        for percentile in self.percentiles.as_mut_slice() {
            let Some(parsed_key) = percentile.intermediate_key.as_ref() else {
                percentile.intermediate_key = Some(key.clone());
                continue;
            };

            let parsed_value = parsed_key.value();
            assert_ne!(value, parsed_value, "Intermediate keys should be unique");

            let old_index = percentile.key_pos;
            let new_index = f32_to_u32(roundf(u64_to_f32(new_count) * percentile.percentile));

            if old_index == new_index {
                //
            }
        }

        use alloc::format;
        debug_print_string(format!(
            "percentiles processing: {value}, {had_existed}, {now_existing}"
        ));

        todo!();
    }
}

fn parse_keys_around(bytes: Option<&[u8]>) -> Option<KeysAround> {
    let Some(bytes) = bytes else {
        return None;
    };

    let mut realign_buffer = take_buffer_for_realign();
    let bytes =
        AlignedBytes::ensure_alignment_or_copy(bytes, realign_buffer.as_mut()).expect("align");
    let data = deserialize::<GetKeysAroundResponseProto>(bytes).expect("parse");

    let left_items = data.left().unwrap_or_default();
    let right_items = data.right().unwrap_or_default();

    let key = parsed_intermediate_key::parse_and_store_intermediate_key(data.key().expect("no key").bytes());
    let mut left = Vec::with_capacity(left_items.len());
    let mut right = Vec::with_capacity(right_items.len());

    for key in left_items {
        let bytes = key.key().expect("no key").bytes();
        left.push(parsed_intermediate_key::parse_and_store_intermediate_key(bytes));
    }
    for key in right_items {
        let bytes = key.key().expect("no key").bytes();
        right.push(parsed_intermediate_key::parse_and_store_intermediate_key(bytes));
    }

    Some((left, key, right))
}

