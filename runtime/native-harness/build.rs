use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let libs = root.join("../../inference-engine/build-local/bin");
    println!("cargo:rustc-link-search=native={}", libs.display());
    println!("cargo:rustc-link-lib=dylib=llama");
    println!("cargo:rustc-link-lib=dylib=ggml-base");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libs.display());
}
