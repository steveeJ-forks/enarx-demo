use std::cell::RefCell;
use std::fs::read;
use std::rc::Rc;
use wasmtime_api::*;
use std::error::Error;
use wasmtime_embed::{wasm_import_wrapper, InstanceToken, RuntimeValue, ContextToken, instantiate_in_context, ImportSet};
use wasmtime_embed_macro::wasm_import;
use std::borrow::Borrow;
use std::collections::HashMap;
use log::{error,trace, warn, info};

mod wasi {
    use super::*;

    /// This trait partially defines WASI
    #[wasm_import]
    pub trait Wasi {
        fn fd_prestat_get(&self, a: u32, b: u32) -> u32;
        fn fd_prestat_dir_name(&self, a: u32, b: u32, c: u32) -> u32;
        fn environ_sizes_get(&self, a: u32, b: u32) -> u32;
        fn environ_get(&self, a: u32, b: u32) -> u32;
        fn args_sizes_get(&self, a: u32, b: u32) -> u32;
        fn args_get(&self, a: u32, b: u32) -> u32;
        fn fd_write(&self, a: u32, b: u32, c: u32, d:u32) -> u32;
        fn proc_exit(&self, a: u32);
        fn fd_fdstat_get(&self, a: u32, b: u32) -> u32;
    }

    pub struct CustomWasi {}

    /// An example implementation of the `Wasi ` trait
    impl Wasi for CustomWasi {
        fn fd_prestat_get(&self, a: u32, b: u32) -> u32 {
            trace!("[fd_prestat_get] a: {}, b: {}", a, b);
            1
        }

        fn fd_prestat_dir_name(&self, a: u32, b: u32, c: u32) -> u32 {
            trace!("[fd_prestat_dir_name] a: {}, b: {}, c: {}", a, b, c);
            1
        }

        fn environ_sizes_get(&self, a: u32, b: u32) -> u32 {
            trace!("[environ_sizes_get] a: {}, b: {}", a, b);
            1
        }

        fn environ_get(&self, a: u32, b: u32) -> u32 {
            trace!("[environ_get] a: {}, b: {}", a, b);
            1
        }

        fn args_sizes_get(&self, a: u32, b: u32) -> u32 {
            trace!("[args_sizes_get] a: {}, b: {}", a, b);
            1
        }

        fn args_get(&self, a: u32, b: u32) -> u32 {
            trace!("[args_get] a: {}, b: {}", a, b);
            1
        }

        fn fd_write(&self, a: u32, b: u32, c: u32, d:u32) -> u32 {
            trace!("[fd_write] a: {}, b: {}, c: {}, d: {}", a, b,c,d);
            1
        }

        fn proc_exit(&self, a: u32) {
            trace!("[proc_exit] a: {}", a);
        }

        fn fd_fdstat_get(&self, a: u32, b: u32) -> u32 {
            trace!("[fd_fdstat_get] a: {}, b: {}", a, b);
            1
        }
    }
}

// static FUNCTION_NAME: &str = "hello_world";

use wasi::{Wasi, CustomWasi};

fn main() -> Result<(), Box<dyn Error>> {
    let _ = env_logger::try_init_from_env(env_logger::Env::default());

    // Before we begin, it'll be heplful to know which source language we're
    // running, so let's print it out.
    if cfg!(feature = "c") {
        info!("WASM application binary has been compiled from C.");
    }
    if cfg!(feature = "rust") {
        info!("WASM appliation binary has been compiled from Rust.");
    }

    // Load and instnatiate the WASM app
    info!("Loading WASM application binary...");
    let app_wasm = read(concat!(env!("OUT_DIR"), "/app.wasm")).unwrap();
    info!("WASM application binary loaded.");

    // Instantiate WASI
    info!("Instantiating WASI from custom implmementation");
    let custom_wasi = CustomWasi{};

    // Call a macro which wraps the struct in a WASM InstanceHandle
    let custom_wasi_wasm = wasm_import_wrapper!(custom_wasi for <CustomWasi as Wasi>);

    // Creat the context and get an instance with the app and WASI together
    let context = ContextToken::create();
    let mut app_wasm_imports = HashMap::new();
    app_wasm_imports.insert(String::from("wasi_unstable"), ImportSet::InstanceExports(custom_wasi_wasm));
    let app_with_wasi = instantiate_in_context(&app_wasm, app_wasm_imports, context).unwrap();

    // Invoke the hello_world function
    app_with_wasi.get_export("hello_world").unwrap().invoke(&[]).unwrap();

    // trace!("the module imports the following functions:");
    // let _function_imports = app_module
    //     .borrow()
    //     .imports()
    //     .iter()
    //     .filter_map(|import| {
    //         if let wasmtime_api::ExternType::ExternFunc(func_type) = import.r#type() {
    //             let module = import.module().to_string();
    //             let name = import.name().to_string();
    //             let params = func_type.params();
    //             let results = func_type.results();

    //             trace!(
    //                 "[{}/{}] parameters: {:?}, results: {:?}",
    //                 module, name, params, results
    //             );

    //             Some((module, name, params, results))
    //         } else {
    //             None
    //         }
    //     })
    //     .collect::<Vec<_>>();

    // module_registry.insert("app".to_owned(), Instance::new(store.clone(), app_module, &[]));

    // Find the index of the hello_world function
    // let hello_world_index = module_registry.get("app")
    //     .unwrap()
    //     .borrow()
    //     .unwrap()
    //     .exports()
    //     .iter()
    //     .enumerate()
    //     .find(|(_index, export)| export.name().to_string() == FUNCTION_NAME)
    //     .ok_or_else(|| {
    //         Err(
    //             format!(
    //                 "Could not find '{}' as an export in the WASM module",
    //                 FUNCTION_NAME
    //             )
    //         ).into()
    //     })?
    //     .0;


    // TODO(steveeJ): instantiate custom WASI

    // TODO(steveeJ): assert that all the function imports are satisfied

    // Instantiate the module.
    // let instance = Rc::new(RefCell::new(Instance::new(store.clone(), module, &[])?));

    // Invoke `hello_world` export
    // let hello_world = instance.borrow().exports()[hello_world_index]
    //     .borrow()
    //     .func()
    //     .clone();

    // let result = hello_world
    //     .borrow()
    //     .call(&[])
    //     .map_err(|e| format_err!("{:?}", e))?;

    // trace!("result: {:#?}", result);

    Ok(())
}
