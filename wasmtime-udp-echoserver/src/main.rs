// Copyright 2019 Red Hat
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::error::Error;

// The basic WASM demo itself.
fn main() -> Result<(), Box<dyn Error>> {
    use wasmtime_api::{
        map_to_wasmtime_trait, wrap_wasmtime_module, Config, Engine, HostRef, Instance, Module,
        Store,
    };

    // Init.
    let engine = HostRef::new(Engine::new(Config::default()));
    let store = HostRef::new(Store::new(&engine));

    // Instantiate the stdio syscalls.
    let syscalls_stdio_module = HostRef::new(wrap_wasmtime_module!(
        &store, |_imports| enarx_syscalls_impl::stdio::Stdio::default(); module(enarx_syscalls_impl::stdio::impl_mod)
    )?);
    let syscalls_stdio_instance = Instance::new(
        &store,
        &syscalls_stdio_module,
        // TODO: what's this array for?
        &[],
    )?;

    // Instantiate the net syscalls.
    let syscalls_net_module = HostRef::new(wrap_wasmtime_module!(
        &store, |_imports| enarx_syscalls_impl::net::Net::default(); module(enarx_syscalls_impl::net::impl_mod)
    )?);
    let syscalls_net_instance = Instance::new(
        &store,
        &syscalls_net_module,
        // TODO: what's this array for?
        &[],
    )?;

    // Load the app's WASM binary.
    let app_wasm_binary = std::fs::read(concat!(env!("OUT_DIR"), "/app.wasm"))?;

    // Compile the app module.
    let app_module = HostRef::new(Module::new(&store, &app_wasm_binary)?);

    // Create the app instance with the enarx_syscalls imported
    let app_instance = {
        // Prepare a lookup-map of module imports and their index
        let app_module_imports = app_module
            .borrow()
            .imports()
            .iter()
            .enumerate()
            .map(|(i, import)| {
                (
                    format!(
                        "{}/{}",
                        import.module().to_string(),
                        import.name().to_string(),
                    ),
                    i,
                )
            })
            .collect::<std::collections::HashMap<_, _>>();
        println!("[RUNTIME] App imports: {:?}", app_module_imports);

        // Instance imports for the app must only include the ones which the module actually references.
        // They need to be in the same order they are referenced.
        // let app_instance_imports = exports_iterator
        let app_instance_imports = std::iter::empty()
            .chain(
                std::iter::repeat(enarx_syscalls_impl::stdio::MODULE_NAME)
                    .zip(syscalls_stdio_instance.exports().iter().cloned())
                    .zip(syscalls_stdio_module.borrow().exports().iter()),
            )
            .chain(
                std::iter::repeat(enarx_syscalls_impl::net::MODULE_NAME)
                    .zip(syscalls_net_instance.exports().iter().cloned())
                    .zip(syscalls_net_module.borrow().exports().iter()),
            )
            .filter_map(|((module_name, ext), exp)| match (&ext, exp) {
                (wasmtime_api::Extern::Func(_), exp) => {
                    // instance imports may only include references which are imported by the module
                    let lookup = format!("{}/{}", module_name, exp.name().to_string());
                    if let Some(index) = app_module_imports.get(&lookup) {
                        println!("[RUNTIME] Including export '{}' at index {}", lookup, index);
                        Some((index, ext))
                    } else {
                        println!("[RUNTIME] Skipping export '{}'", lookup);
                        None
                    }
                }
                // TODO: figure out what to do with non-Func externs
                _ => None,
            })
            // ensure the same order as observed in the imports
            .collect::<std::collections::BTreeMap<_, _>>()
            .values()
            .cloned()
            .collect::<Vec<wasmtime_api::Extern>>();

        HostRef::new(Instance::new(
            &store,
            &app_module,
            app_instance_imports.as_slice(),
        )?)
    };

    // Map the trait to the App module
    let app_wrapper = map_to_wasmtime_trait!(app_instance; module(app_mod));

    // Call its main function
    app_wrapper.main();

    Ok(())
}

use wasmtime_bindings_macro::wasmtime_trait;
#[wasmtime_trait(module(app_mod))]
trait App {
    fn main(&self);
}

pub(crate) mod enarx_syscalls_impl {
    use common::wasm::wasmstr::WasmStr;
    use std::convert::TryInto;
    use wasmtime_bindings_macro::wasmtime_impl;

    pub(crate) mod stdio {
        use super::*;

        pub static MODULE_NAME: &str = "enarx_syscalls_stdio";
        pub struct Stdio;

        impl Default for Stdio {
            fn default() -> Self {
                Self
            }
        }

        #[wasmtime_impl(
            module(impl_mod),
            // we're passing a context type here which satisfies `impl WasmMem`
            context(wasmtime_wasi::WasiMem),
            visibility(pub),
        )]
        impl Stdio {
            fn __print_str(&self, ptr: *const u8, len: u64) {
                let wasm_str = WasmStr(ptr, len);
                print!("[WASM] {}", wasm_str.to_str());
            }
        }
    }

    pub(crate) mod net {
        use super::*;

        pub static MODULE_NAME: &str = "enarx_syscalls_net";

        pub struct Net {
            pub udp_socket: Option<std::net::UdpSocket>,
        }

        impl Default for Net {
            fn default() -> Self {
                Net { udp_socket: None }
            }
        }

        #[wasmtime_impl(
            module(impl_mod),
            // we're passing a context type here which satisfies `impl WasmMem`
            context(wasmtime_wasi::WasiMem),
            visibility(pub),
        )]
        impl Net {
            fn __socket_udp_bind(&mut self, ptr: *const u8, len: u64) {
                let addr_str = WasmStr(ptr, len);
                self.udp_socket = Some(std::net::UdpSocket::bind(addr_str.to_str()).unwrap());
            }

            fn __socket_udp_receive(
                &mut self,
                data_ptr: *mut u8,
                data_len: u64,
                src_ptr: *mut u8,
                src_len: u64,
            ) -> u64 {
                let mut data_buf = unsafe {
                    std::slice::from_raw_parts_mut(data_ptr, data_len.try_into().unwrap())
                };

                if let Some(socket) = self.udp_socket.as_ref() {
                    println!("[RUNTIME] receiving on {:?}", socket);
                    let (amt, src) = socket.recv_from(&mut data_buf).unwrap();
                    println!("[RUNTIME] received {} bytes from {}", amt, src);

                    let src_str = format!("{}:{}\0", src.ip(), src.port());

                    let src_str_size = std::mem::size_of_val(&src_str);

                    assert!(src_str_size <= src_len.try_into().unwrap());

                    unsafe {
                        std::ptr::copy(src_str.as_ptr(), src_ptr, src_str_size);
                    };

                    amt.try_into().unwrap()
                } else {
                    0
                }
            }

            fn __socket_udp_send_to(
                &mut self,
                data_ptr: *const u8,
                data_len: u64,
                dst_ptr: *const u8,
                dst_len: u64,
            ) -> u64 {
                let data =
                    unsafe { std::slice::from_raw_parts(data_ptr, data_len.try_into().unwrap()) };
                let dst = WasmStr(dst_ptr, dst_len.try_into().unwrap());

                if let Some(socket) = self.udp_socket.as_ref() {
                    let amt = socket.send_to(&data, dst.to_str()).unwrap();
                    println!("[RUNTIME] sent {} bytes to {}", amt, dst.to_str());
                    amt.try_into().unwrap()
                } else {
                    0
                }
            }
        }
    }
}