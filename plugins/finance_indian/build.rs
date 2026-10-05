fn main() {
    println!("cargo:rustc-check-cfg=cfg(lariv_plugin_crate)");
    println!("cargo:rustc-cfg=lariv_plugin_crate");
}
