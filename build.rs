fn main() {
    let build_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs();
    println!("cargo:rustc-env=VERITAS_BUILD_ID={build_id}");

    let ver = env!("CARGO_PKG_VERSION").split(".").map(|x| x.parse::<u64>().unwrap()).collect::<Vec<u64>>();
    let sem_ver = ver[0] << 48 | ver[1] << 32 | ver[1] << 16;

    // cc emits rerun-if directives, which would otherwise stop this script (and
    // VERITAS_BUILD_ID) from rerunning on source changes.
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
    cc::Build::new().file("src/guard.c").compile("veritas_guard");

    winres::WindowsResource::new()
        .set_version_info(winres::VersionInfo::PRODUCTVERSION, sem_ver)
        .compile()
        .unwrap();
}