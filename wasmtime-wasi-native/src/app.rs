fn main() {}

#[no_mangle]
pub extern fn hello_world() {
    println!("Hello, world!");
    panic!("this can't work");
}