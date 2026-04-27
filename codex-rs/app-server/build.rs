fn main() {
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rerun-if-changed=src/devicecheck_probe.m");
        cc::Build::new()
            .file("src/devicecheck_probe.m")
            .flag("-fblocks")
            .compile("codex_devicecheck_probe");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
}
