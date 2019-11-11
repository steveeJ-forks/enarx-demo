# Wasmtime UDP Echo-Server Demo
This demo shows the extension of the previously introduced _stdio_ syscall
module, and adds a _net_ syscall module, which implements rudimentary
UDP functionality.
The WASM guest application uses these syscalls to build an UDP echo
server.

The WASM application lives in _app.rs_, and is compiled as per the
instructions in _build.rs_.

## Running the demo
The demo runs with `cargo run` out of the box.
