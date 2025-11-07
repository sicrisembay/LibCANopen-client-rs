// Build script to configure PEAK CAN library linking

use std::env;
use std::path::PathBuf;

fn main() {
    // Get the project directory
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let peak_lib_path = PathBuf::from(&manifest_dir)
        .join("src")
        .join("hardware")
        .join("Peak")
        .join("x64")
        .join("VC_LIB");

    // Tell Rust where to find the PEAK CAN library
    println!("cargo:rustc-link-search=native={}", peak_lib_path.display());
    
    // Link against the PEAK CAN library
    println!("cargo:rustc-link-lib=static=PCANBasic");
    
    // Tell Cargo to re-run this script if the library files change
    let peak_lib_file = peak_lib_path.join("PCANBasic.lib");
    println!("cargo:rerun-if-changed={}", peak_lib_file.display());
    
    println!("cargo:rerun-if-changed=build.rs");
    
    // Optional: Print the library path for debugging
    println!("cargo:warning=Using PEAK CAN library from: {}", peak_lib_path.display());
}