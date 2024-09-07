extern crate bindgen;

use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use bindgen::callbacks::{MacroParsingBehavior, ParseCallbacks};

#[derive(Debug)]
struct MacroCallback {
    macros: Arc<RwLock<HashSet<String>>>,
}

impl ParseCallbacks for MacroCallback {
    fn will_parse_macro(&self, name: &str) -> bindgen::callbacks::MacroParsingBehavior {
        self.macros.write().unwrap().insert(name.into());
        match name {
            "FP_NAN" | "FP_INFINITE" | "FP_ZERO" | "FP_NORMAL" | "FP_SUBNORMAL" => {
                MacroParsingBehavior::Ignore
            }
            _ => MacroParsingBehavior::Default,
        }
    }
}

fn main() {
    let pwd = env::var("CARGO_MANIFEST_DIR").unwrap();
    let vendor_path = Path::new(&pwd).join("vendor");
    let pwd_path = Path::new(&pwd);
    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR env var not set?"));
    let aztro_core_path = PathBuf::from(vendor_path);
    let clang_arg = format!("-I{}", aztro_core_path.to_string_lossy());

    let mut build = cc::Build::new();

    if cfg!(target_os = "windows") {
        build.flag("/W4");
    } else {
        build.flag("-g")
            .flag("-Wall")
            .flag("-fPIC");
    }

    build.files([
        pwd_path.join("vendor/swecl.c"),
        pwd_path.join("vendor/swedate.c"),
        pwd_path.join("vendor/swehel.c"),
        pwd_path.join("vendor/swehouse.c"),
        pwd_path.join("vendor/swejpl.c"),
        pwd_path.join("vendor/swemmoon.c"),
        pwd_path.join("vendor/swemplan.c"),
        pwd_path.join("vendor/sweph.c"),
        pwd_path.join("vendor/swephlib.c"),
    ])
    .compile("swe");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=src/wrapper.h");
    println!("cargo:rerun-if-changed=vendor/swecl.c");
    println!("cargo:rerun-if-changed=vendor/swedate.c");
    println!("cargo:rerun-if-changed=vendor/swehel.c");
    println!("cargo:rerun-if-changed=vendor/swehouse.c");
    println!("cargo:rerun-if-changed=vendor/swejpl.c");
    println!("cargo:rerun-if-changed=vendor/swemmoon.c");
    println!("cargo:rerun-if-changed=vendor/swemplan.c");
    println!("cargo:rerun-if-changed=vendor/sweph.c");
    println!("cargo:rerun-if-changed=vendor/swephlib.c");

    println!("cargo:rustc-link-search={}", aztro_core_path.to_string_lossy());
    println!("cargo:rustc-link-lib=swe");

    let macros = Arc::new(RwLock::new(HashSet::new()));

    let bindings = bindgen::Builder::default()
        .header("src/wrapper.h")
        .clang_arg(clang_arg)
        .parse_callbacks(Box::new(MacroCallback {
            macros: macros.clone(),
        }))
        .allowlist_function("swe_.*")
        .allowlist_var("SE.*")
        .generate()
        .expect("Unable to generate bindings.");

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Unable to write bindings.");
}