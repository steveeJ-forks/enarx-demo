//! This is an example crate demonstrating how Enarx may use Wasmtime, a
//! Rust-powered JIT, to natively run programs from several different source
//! languages (Rust/C) compiled to WASI-compliant WASM.

use failure::{format_err, Fallible};
use std::cell::RefCell;
use std::fs::read;
use std::rc::Rc;
use wasmtime_api::*;

mod wasi;

fn main() -> Fallible<()> {
    // Before we begin, it'll be heplful to know which source language we're
    // running, so let's print it out.
    if cfg!(feature = "c") {
        println!("WASM binary has been compiled from C.");
    }
    if cfg!(feature = "rust") {
        println!("WASM binary has been compiled from Rust.");
    }

    static FUNCTION_NAME: &str = "hello_world";

    println!("Loading WASM binary...");
    let binary = read(concat!(env!("OUT_DIR"), "/source.wasm")).unwrap();
    println!("WASM binary loaded.");

    // Instantiate engine and store.
    let engine = Rc::new(RefCell::new(Engine::default()));
    let store = Rc::new(RefCell::new(Store::new(engine)));
    let mut module_registry = std::collections::HashMap::new();

    // Load the app module.
    let app_module = Rc::new(RefCell::new(Module::new(store.clone(), &binary)?));

    module_registry.insert("app".to_owned(), Instance::new(store.clone(), app_module, &[]));

    // Find the index of the hello_world function
    // let hello_world_index = module
    //     .borrow()
    //     .exports()
    //     .iter()
    //     .enumerate()
    //     .find(|(_index, export)| export.name().to_string() == FUNCTION_NAME)
    //     .ok_or_else(|| {
    //         format_err!(
    //             "Could not find '{}' as an export in the WASM module",
    //             FUNCTION_NAME
    //         )
    //     })?
    //     .0;

    println!("the module imports the following functions:");
    let _function_imports = app_module
        .borrow()
        .imports()
        .iter()
        .filter_map(|import| {
            if let wasmtime_api::ExternType::ExternFunc(func_type) = import.r#type() {
                let module = import.module().to_string();
                let name = import.name().to_string();
                let params = func_type.params();
                let results = func_type.results();

                println!(
                    "[{}/{}] parameters: {:?}, results: {:?}",
                    module, name, params, results
                );

                Some((module, name, params, results))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // TODO(steveeJ): instantiate custom WASI
    let wasi_instance = wasmtime_wasi::instantiate_wasi(
        "",
        Default::default(),
        Default::default(),
        Default::default(),
        Default::default(),
    )?;

    // TODO(steveeJ): assert that all the function imports are satisfied

    // Instantiate the module.
    let instance = Rc::new(RefCell::new(Instance::new(store.clone(), module, &[])?));

    // Invoke `hello_world` export
    let hello_world = instance.borrow().exports()[hello_world_index]
        .borrow()
        .func()
        .clone();

    let result = hello_world
        .borrow()
        .call(&[])
        .map_err(|e| format_err!("{:?}", e))?;

    println!("result: {:#?}", result);

    Ok(())
}
