use crate::commands::errors::CommandError;
use diffbelt_cli_config::CliConfig;
use diffbelt_http_client::client::DiffbeltClient;
use diffbelt_util::cast::checked_usize_to_i32;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct CliState {
    pub client: Arc<DiffbeltClient>,
    pub config: Option<Rc<CliConfig>>,
    pub verbose: bool,
    exit_code_atomic: AtomicUsize,
}

impl CliState {
    pub fn new(client: DiffbeltClient, config: Option<Rc<CliConfig>>, verbose: bool) -> Self {
        Self {
            client: Arc::new(client),
            config,
            verbose,
            exit_code_atomic: AtomicUsize::new(0),
        }
    }

    pub fn exit_code(&self) -> i32 {
        let code = self.exit_code_atomic.load(Ordering::Relaxed);

        checked_usize_to_i32(code)
    }

    pub fn set_non_zero_exit_code(&self, code: usize) {
        loop {
            // Can use Relaxed, because not depends on any order variables
            let result = self.exit_code_atomic.compare_exchange_weak(
                0,
                code,
                Ordering::Relaxed,
                Ordering::Relaxed,
            );

            let Err(value) = result else {
                // was changed from 0 to code
                break;
            };

            if value != 0 {
                break;
            }
        }
    }

    pub fn require_config(&self) -> Result<&CliConfig, CommandError> {
        let Some(config) = &self.config else {
            return Err(CommandError::Message("Specify config path with --config parameter\n\nExample: diffbelt_cli --config config.yaml test".to_string()));
        };

        Ok(config.deref())
    }
}
