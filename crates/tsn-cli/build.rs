fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let stdlib_root = std::path::Path::new(&manifest_dir)
        .join("..")
        .join("..")
        .join("tsn-stdlib");

    let protos: Vec<tsn_compiler::FunctionProto> = Vec::new();
    for spec in tsn_modules::BUILTIN_MODULES {
        let name = spec
            .strip_prefix("builtin:")
            .unwrap_or_else(|| panic!("invalid builtin spec '{spec}'"));
        let path = stdlib_root.join("builtins").join(format!("{name}.tsn"));
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let bytes = bincode::serialize(&protos).expect("cannot serialize builtin protos");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not set");
    let out_path = std::path::Path::new(&out_dir).join("builtins.bin");
    std::fs::write(&out_path, bytes).expect("cannot write builtins.bin");
}
