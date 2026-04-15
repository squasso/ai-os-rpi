fn main() {
    // Attiva le feature wayland/x11 automaticamente su Linux
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo:rustc-cfg=feature=\"linux-display\"");
    }
}
