#[no_mangle]
pub extern "C" fn process_heavy_task() {
    println!("Processamento FFI em Rust chamado pelo Go.");
    let v1 = vec![2, 3, 1];
    let v2 = vec![1, 1, 2];

    let produto_escalar: i32 = v1
        .iter()
        .zip(v2.iter())
        .map(|(a, b)| a * b)
        .sum();

    println!("Resultado: {}", produto_escalar);
}
