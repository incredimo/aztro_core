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
                return MacroParsingBehavior::Ignore
            }
            _ => MacroParsingBehavior::Default,
        }
    }
}

fn main() {
    let pwd = env::var("CARGO_MANIFEST_DIR").unwrap();
    let pwd_path = Path::new(&pwd);
    let out_path = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR env var not set?"));
    let cosmo_ephemeris_path =
        PathBuf::from(env::var("RUST_cosmo_ephemeris_SYS_SOURCE").unwrap_or("vendor".to_owned()));
    let clang_arg = format!("-I{}", cosmo_ephemeris_path.to_string_lossy());

    let mut build = cc::Build::new();

    // Set different flags based on the target OS
    if cfg!(target_os = "windows") {
        build.flag("/W4"); // Warning level 4
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
    println!("cargo:rustc-link-search={}", cosmo_ephemeris_path.to_string_lossy());
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