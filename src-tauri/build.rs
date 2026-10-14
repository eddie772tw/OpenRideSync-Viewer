fn main() {
    println!("cargo:rerun-if-env-changed=OPENRIDESYNC_SERVICE_CONFIG");
    let config = std::env::var("OPENRIDESYNC_SERVICE_CONFIG").unwrap_or_else(|_| "null".into());
    let config = if config.trim().is_empty() {
        "null"
    } else {
        &config
    };
    std::fs::write(
        std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("service-config.json"),
        config,
    )
    .expect("Unable to prepare build-time service configuration");
    tauri_build::build()
}
