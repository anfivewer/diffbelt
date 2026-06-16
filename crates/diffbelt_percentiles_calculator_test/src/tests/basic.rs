use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use diffbelt_percentiles_calculator::{
    calculator_impl::CalculatorImpl,
    types::{DiffKey, PercentileFull, PercentilesCalculator, PercentilesCalculatorOptions, PercentilesTargetRecord},
};

use crate::tests::{
    data_provider::{MockDataProvider, MockDataProviderOptions},
    MockPTypes, MockSourceChunk, MockSourceRecord,
};

#[test]
fn test_percentiles_basic() {
    let rng = Arc::new(Mutex::new(ChaCha8Rng::seed_from_u64(0x9a9ddd206ce854ef)));

    // Empty initial data
    let data_provider = MockDataProvider::<MockPTypes>::new(MockDataProviderOptions {
        initial_target_records: HashMap::new(),
        initial_source_keys: Vec::new(),
    });

    let data_provider_clone = data_provider.clone();
    let rng_clone = rng.clone();

    // Spawn thread for calculator loop
    let calc_handle = thread::spawn(move || -> Result<(), diffbelt_percentiles_calculator::types::PercentilesError<MockPTypes>> {
        // Generate total number of chunks before loop
        let total_chunks = {
            let mut rng_guard = rng_clone.lock().unwrap();
            rng_guard.gen_range(1..=64)
        };

        let mut actual_source_keys: HashMap<Arc<[u8]>, Vec<Arc<[u8]>>> = HashMap::new();
        let percentiles_to_calc = vec![0.0, 0.5, 0.75, 1.0];

        for _ in 0..total_chunks {
            // Generate random source chunk (hold lock only for generation)
            let source_chunk = {
                let mut rng_guard = rng_clone.lock().unwrap();
                let num_records = rng_guard.gen_range(0..100);
                let mut records = Vec::with_capacity(num_records);

                for _ in 0..num_records {
                    let mut key_bytes = [0u8; 4];
                    rng_guard.fill(&mut key_bytes);
                    let diff_key = if rng_guard.gen_bool(0.7) {
                        DiffKey::Added(Arc::from(key_bytes.as_slice()))
                    } else {
                        DiffKey::Removed(Arc::from(key_bytes.as_slice()))
                    };

                    records.push(MockSourceRecord { diff_key });
                }

                MockSourceChunk { records }
            };
            // Lock released here before calling calculate

            // Update actual_source_keys
            for record in &source_chunk.records {
                let source_key = match &record.diff_key {
                    DiffKey::Added(k) | DiffKey::Removed(k) => k,
                };
                let target_key: Arc<[u8]> = Arc::from(&[source_key[0] % 8][..]);

                let vec = actual_source_keys.entry(target_key).or_default();
                match &record.diff_key {
                    DiffKey::Added(k) => {
                        let pos = vec.binary_search_by(|existing| existing.as_ref().cmp(k.as_ref())).unwrap_or_else(|e| e);
                        vec.insert(pos, k.clone());
                    }
                    DiffKey::Removed(k) => {
                        if let Ok(pos) = vec.binary_search_by(|existing| existing.as_ref().cmp(k.as_ref())) {
                            vec.remove(pos);
                        }
                    }
                }
            }

            let options = PercentilesCalculatorOptions::<MockPTypes> {
                chunk: source_chunk,
                data_provider: data_provider_clone.clone(),
                source_key_to_target_key: Box::new(|key: &Arc<[u8]>| Arc::from(&[key[0] % 8][..])),
                percentiles: percentiles_to_calc.clone(),
            };

            let mut calculator = CalculatorImpl::new(options);

            // Call calculate - may block waiting for data provider
            if let Err(e) = calculator.calculate() {
                return Err(e);
            }
        }

        let all_target_keys = data_provider_clone.get_all_target_keys();
        
        let actual_keys_set: HashSet<_> = actual_source_keys.keys().collect();
        let expected_keys_set: HashSet<_> = all_target_keys.iter().collect();
        assert_eq!(actual_keys_set, expected_keys_set, "Target keys mismatch");

        for target_key in all_target_keys {
            let record = data_provider_clone.get_target_record(&target_key).expect("record should exist");
            let source_keys = actual_source_keys.get(&target_key).unwrap();
            
            let expected_percentiles: Vec<PercentileFull<MockPTypes>> = percentiles_to_calc
                .iter()
                .map(|&p| {
                    let key = if source_keys.is_empty() {
                        None
                    } else {
                        let idx = (p * (source_keys.len() - 1) as f32).round() as usize;
                        Some(source_keys[idx].clone())
                    };
                    PercentileFull { p, key }
                })
                .collect();
            
            let actual_percentiles = record.percentiles();
            assert_eq!(
                actual_percentiles.len(),
                expected_percentiles.len(),
                "Percentiles length mismatch for target key {:?}",
                target_key
            );
            for (actual, expected) in actual_percentiles.iter().zip(expected_percentiles.iter()) {
                assert!(
                    (actual.p - expected.p).abs() < 1e-6,
                    "Percentile p mismatch: {} vs {} for target key {:?}",
                    actual.p,
                    expected.p,
                    target_key
                );
                assert_eq!(
                    actual.key,
                    expected.key,
                    "Percentile key mismatch for p={} and target key {:?}",
                    expected.p,
                    target_key
                );
            }
        }

        Ok(())
    });

    // Main thread controls data provider
    loop {
        // Check if calculator thread finished
        if calc_handle.is_finished() {
            break;
        }

        if data_provider.await_some_pending_tasks(Duration::from_secs(1)) {
            let mut rng_guard = rng.lock().unwrap();
            data_provider.resolve_random_async(&mut *rng_guard);
        }
    }

    let result = calc_handle.join().expect("calculator thread panicked");
    assert!(result.is_ok());

    // Assert that all pending tasks are resolved
    assert_eq!(data_provider.pending_asyncs_count(), 0);
}