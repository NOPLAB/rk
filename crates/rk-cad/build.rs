fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // OCCT's OSD layer calls the Windows security, registry and user-name
    // APIs, all of which live in advapi32 — and opencascade-sys never asks
    // for it. Every binary linking rk-cad then fails on two dozen
    // unresolved symbols. It went unnoticed only because the egui frontend
    // was the one thing that ever linked this kernel, and winit pulled
    // advapi32 in for reasons of its own.
    let windows = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    if windows && std::env::var_os("CARGO_FEATURE_OPENCASCADE").is_some() {
        println!("cargo:rustc-link-lib=dylib=advapi32");
    }
}
