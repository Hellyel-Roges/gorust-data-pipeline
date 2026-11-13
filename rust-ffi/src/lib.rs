#[no_mangle]
pub extern "C" fn process_heavy_task() {
    println!("Processamento FFI em Rust chamado pelo Go.");
}
