fn main() {
    // Check target OS via environment variable (works for cross-compilation)
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();

        // Set windres path for cross-compilation from Linux
        if std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default() == "x86_64" {
            res.set_windres_path("x86_64-w64-mingw32-windres");
        }

        res.set_icon("../../assets/icons/rk.ico");
        res.compile().expect("Failed to compile Windows resources");
    }
}
