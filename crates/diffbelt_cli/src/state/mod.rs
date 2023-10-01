use std::rc::Rc;
use diffbelt_http_client::client::DiffbeltClient;
use diffbelt_util::cast::checked_usize_to_i32;
use std::sync::atomic::{AtomicUsize, Ordering};
use diffbelt_cli_config::CliConfig;

pub struct CliState {
    pub client: DiffbeltClient,
    pub config: Option<Rc<CliConfig>>,
    exit_code_atomic: AtomicUsize,
}

impl CliState {
    pub fn new(client: DiffbeltClient, config: Option<Rc<CliConfig>>) -> Self {
        Self {
            client,
            config,
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
}
