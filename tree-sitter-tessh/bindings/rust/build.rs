fn main() {
    let src = std::path::Path::new("src");
    cc::Build::new().include(src).file(src.join("parser.c")).flag_if_supported("-Wno-unused-parameter").compile("tree-sitter-tessh");
    println!("cargo:rerun-if-changed=src/parser.c");
}
