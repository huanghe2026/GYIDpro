fn main() {
    // 从 UDL 生成 UniFFI scaffolding（OUT_DIR/gyid.uniffi.rs）
    uniffi::generate_scaffolding("./src/gyid.udl").unwrap();
}
