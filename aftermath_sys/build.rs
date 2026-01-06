pub fn copy_dir_all(
    src: impl AsRef<std::path::Path>,
    dst: impl AsRef<std::path::Path>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn main() {
    println!("cargo:rerun-if-changed=./build.rs");

    let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let from_lib_path = std::path::PathBuf::from("./vendor/current/lib")
        .canonicalize()
        .unwrap();
    let to_lib_path = out_path.join("lib");
    let _ = copy_dir_all(from_lib_path, &to_lib_path).unwrap();

    println!("cargo:rustc-link-lib={}", lib_name());

    #[cfg(target_arch = "x86_64")]
    {
        println!(
            "cargo:rustc-link-search={}",
            to_lib_path.join("x64").display()
        );
    };
    #[cfg(target_arch = "x86")]
    {
        println!(
            "cargo:rustc-link-search={}",
            to_lib_path.join("x86").display()
        );
    };

    /*
        NOTE: Currently commented out as I switched to shipping pre-generated bindings
              See `./generate.sh`
    */
    // generate_bindings();
}

#[allow(unreachable_code)]
pub const fn lib_name() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        return "GFSDK_Aftermath_Lib.x64";
    };

    #[cfg(target_arch = "x86")]
    {
        return "GFSDK_Aftermath_Lib.x86";
    };

    {
        panic!();
    };
}

// pub fn generate_bindings() {
//     let bindings = bindgen::Builder::default()
//         .headers([
//             "./vendor/current/include/GFSDK_Aftermath.h",
//             "./vendor/current/include/GFSDK_Aftermath_Defines.h",
//             "./vendor/current/include/GFSDK_Aftermath_GpuCrashDump.h",
//             "./vendor/current/include/GFSDK_Aftermath_GpuCrashDumpDecoding.h",
//             "./vendor/current/include/GFSDK_Aftermath_GpuCrashDumpEditing.h",
//         ])
//         .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
//         .use_core()
//         .generate()
//         .unwrap();
//     let out_path = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
//     // let out_path = std::path::PathBuf::from("./src/");
//     let _ = bindings
//         .write_to_file(out_path.join("bindings.rs"))
//         .unwrap();
// }
