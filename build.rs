fn main() {
    // Pure static binary, no external C/C++ deps needed
    // Target: aarch64-linux-android (via cargo-ndk or cross-compile)
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=build.rs");
}
