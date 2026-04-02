fn main() {
    const BUILTINS: &[&str] = &["builtin:global", "builtin:primitives", "builtin:classes"];
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let stdlib_root = std::path::Path::new(&manifest_dir)
        .join("..")
        .join("..")
        .join("tsn-stdlib");

    let mut protos = Vec::with_capacity(BUILTINS.len());
    for spec in BUILTINS {
        let name = spec
            .strip_prefix("builtin:")
            .unwrap_or_else(|| panic!("invalid builtin spec '{spec}'"));
        let path = stdlib_root.join("builtins").join(format!("{name}.tsn"));
        if !path.is_file() {
            panic!(
                "cannot resolve stdlib builtin path for '{spec}' at {}",
                path.display()
            );
        }
        println!("cargo:rerun-if-changed={}", path.display());

        let src = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read builtin '{spec}' at {}: {e}", path.display()));
        let filename = path.to_string_lossy();
        let tokens = tsn_lexer::scan(&src, &filename);
        let program = tsn_parser::parse(tokens, &filename)
            .unwrap_or_else(|e| panic!("cannot parse builtin '{spec}': {e:?}"));
        let proto = tsn_compiler::compile(&program)
            .unwrap_or_else(|e| panic!("cannot compile builtin '{spec}': {e}"));
        protos.push(proto);
    }

    let bytes = bincode::serialize(&protos).expect("cannot serialize builtin protos");
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is not set");
    let out_path = std::path::Path::new(&out_dir).join("builtins.bin");
    std::fs::write(&out_path, bytes).expect("cannot write builtins.bin");
}
