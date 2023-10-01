use diffbelt_cli_config::CliConfig;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static CLI_CONFIG: RefCell<Option<Rc<CliConfig>>> = RefCell::new(None);
}

pub fn set_global_config(config: Rc<CliConfig>) {
    CLI_CONFIG.with(|value| {
        let mut cfg = value.borrow_mut();
        cfg.replace(config);
    });
}

pub fn get_global_config() -> Option<Rc<CliConfig>> {
    CLI_CONFIG.with(|value| {
        let cfg = value.borrow();

        cfg.clone()
    })
}
