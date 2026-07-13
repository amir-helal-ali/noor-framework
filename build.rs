// ============================================================
// build.rs - Build script for Zig performance modules
// سكربت البناء لمكونات Zig عالية الأداء
// ============================================================
// This script compiles Zig source files into a static library
// and links it with the Rust binary. The Zig modules handle
// performance-critical paths like HTTP parsing and crypto.
//
// يقوم هذا السكربت بتجميع ملفات Zig إلى مكتبة ثابتة
// وربطها بالـ Rust binary.
// ============================================================

use std::process::Command;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/zig/");
    
    let zig_dir = PathBuf::from("src/zig");
    let output_lib = PathBuf::from("target/zig-build/libnoor_zig.a");
    
    // Try to compile Zig modules if zig is available
    let zig_available = Command::new("zig")
        .arg("version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    
    if zig_available {
        println!("cargo:warning=Zig compiler found, compiling performance modules...");
        
        // Create output directory
        std::fs::create_dir_all("target/zig-build").ok();
        
        // Compile Zig to static library
        let status = Command::new("zig")
            .args(&[
                "build-lib",
                "-dynamic",
                "-O", "ReleaseFast",
                "-femit-bin=target/zig-build/libnoor_zig.so",
                "src/zig/noor_zig.zig",
            ])
            .status();
        
        match status {
            Ok(s) if s.success() => {
                println!("cargo:warning=Zig modules compiled successfully");
                println!("cargo:rustc-link-search=native=target/zig-build");
                println!("cargo:rustc-link-lib=dylib=noor_zig");
            }
            _ => {
                println!("cargo:warning=Zig compilation failed, using Rust fallback");
            }
        }
    } else {
        println!("cargo:warning=Zig compiler not found, using pure Rust implementation");
    }
}
