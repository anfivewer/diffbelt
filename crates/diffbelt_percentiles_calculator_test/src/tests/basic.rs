use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use diffbelt_percentiles_calculator::{
    calculator_impl::CalculatorImpl,
    types::{DiffKey, PercentilesCalculator, PercentilesCalculatorOptions},
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

            let options = PercentilesCalculatorOptions::<MockPTypes> {
                chunk: source_chunk,
                data_provider: data_provider_clone.clone(),
            };

            let mut calculator = CalculatorImpl::new(options);

            // Call calculate - may block waiting for data provider
            if let Err(e) = calculator.calculate() {
                return Err(e);
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