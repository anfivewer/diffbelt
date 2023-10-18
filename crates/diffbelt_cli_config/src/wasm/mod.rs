use crate::errors::WithMark;
use serde::Deserialize;
use std::cell::{Ref, RefCell, RefMut};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::rc::Rc;
use wasmer::{imports, CompileError, Instance, InstantiationError, Module, Store};

#[derive(Deserialize, Debug)]
pub struct Wasm {
    pub name: Rc<str>,
    pub wat_path: WithMark<String>,
    #[serde(skip)]
    wasm_store_: RefCell<Option<Store>>,
    #[serde(skip)]
    wasm_module_: RefCell<Option<Module>>,
    #[serde(skip)]
    wasm_instance_: RefCell<Option<Instance>>,
}

#[derive(Debug)]
pub enum WasmError {
    AlreadyErrored,
    Io(std::io::Error),
    Compile(CompileError),
    Instantiation(InstantiationError),
}

pub struct GetWasmInstanceCall<'a> {
    pub config_path: &'a str,
    pub wasm: &'a Wasm,
}

impl<'a> GetWasmInstanceCall<'a> {
    pub async fn call(self) -> Result<impl Deref<Target = Instance> + 'a, WasmError> {
        let GetWasmInstanceCall { config_path, wasm } = self;

        'if_presented: {
            let instance = wasm.wasm_instance_.borrow();

            let Ok(instance) = Ref::filter_map(instance, |x| x.as_ref()) else {
                break 'if_presented;
            };

            return Ok(instance);
        }

        let mut wat_path =
            PathBuf::with_capacity(config_path.as_bytes().len() + 1 + wasm.name.as_bytes().len());
        wat_path.push(config_path);
        wat_path.push(wasm.name.deref());

        let wat_bytes = tokio::fs::read(wat_path).await.map_err(WasmError::Io)?;

        // Check again, because after await it can be updated
        'if_presented: {
            let instance = wasm.wasm_instance_.borrow();

            let Ok(instance) = Ref::filter_map(instance, |x| x.as_ref()) else {
                break 'if_presented;
            };

            return Ok(instance);
        }

        {
            let s = wasm.wasm_store_.borrow_mut();
            let mut s = match RefMut::filter_map(s, |s| s.as_mut()) {
                Ok(s) => s,
                Err(mut s) => {
                    let store = Store::default();
                    s.replace(store);

                    RefMut::filter_map(s, |s| s.as_mut()).unwrap()
                }
            };

            let store = s.deref();

            let wasm_mod = Module::new(store, wat_bytes).map_err(WasmError::Compile)?;

            let mut m = wasm.wasm_module_.borrow_mut();
            let m = m.deref_mut();

            m.replace(wasm_mod);
            let wasm_mod = m.as_ref().unwrap();

            let store = s.deref_mut();

            let import_object = imports! {};

            let instance =
                Instance::new(store, wasm_mod, &import_object).map_err(WasmError::Instantiation)?;

            let mut inst = wasm.wasm_instance_.borrow_mut();
            inst.replace(instance);
        }

        let instance = wasm.wasm_instance_.borrow();
        let instance = Ref::filter_map(instance, |x| x.as_ref()).unwrap();

        Ok(instance)
    }
}
