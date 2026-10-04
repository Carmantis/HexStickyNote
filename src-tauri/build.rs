fn main() {
    // Lets dev builds find the HexTime sidecar as binaries/hextime-server-<triple>
    println!(
        "cargo:rustc-env=TARGET_TRIPLE={}",
        std::env::var("TARGET").expect("cargo sets TARGET for build scripts")
    );

    tauri_build::build()
}
