pub const fn lib_name() -> &'static str {
    #[cfg(windows)]
    {
        return "GFSDK_Aftermath_Lib.x86.dll";
    };

    #[cfg(unix)]
    {
        return "GFSDK_Aftermath_Lib.x86.lib";
    };

    #[allow(unreachable_code)]
    {
        panic!();
    };
}

fn main() {
    println!("cargo:rerun-if-changed=./build.rs");

    let bindings = bindgen::Builder::default()
        .headers([
            "./vendor/current/include/GFSDK_Aftermath.h",
            "./vendor/current/include/GFSDK_Aftermath_Defines.h",
            "./vendor/current/include/GFSDK_Aftermath_GpuCrashDump.h",
            "./vendor/current/include/GFSDK_Aftermath_GpuCrashDumpDecoding.h",
        ])
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .use_core()
        .generate()
        .unwrap();

    // println!("cargo:rustc-llink-search=./vendor/current/lib/x64/");
    println!("cargo:rustc-llink-search=./vendor/current/lib/x86/");
    println!("cargo:rustc-link-lib={}", lib_name());

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    // let out_path = std::path::PathBuf::from("./src/");

    let _ = bindings
        .write_to_file(out_path.join("bindings.rs"))
        .unwrap();
}
