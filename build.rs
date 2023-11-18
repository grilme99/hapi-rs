use std::path::Path;

fn main() {
    let build_enabled = env::var("HAPI_BUILD_ENABLED")
        .map(|v| v == "1")
        .unwrap_or(true);
    
    if !build_enabled {
        return
    }
    
    if std::env::var("DOCS_RS").is_ok() {
        return;
    }
    let hfs = std::env::var("HFS").expect("HFS variable not set");
    let filename;
    let lib_dir;
    if cfg!(target_os = "macos") {
        filename = "HAPIL";
        let _lib_dir = Path::new(&hfs).parent().unwrap().join("Libraries");
        lib_dir = _lib_dir.to_string_lossy().to_string();
    } else if cfg!(target_os = "windows") {
        filename = "libHAPIL";
        lib_dir = format!("{}/custom/houdini/dsolib", hfs);
    } else {
        filename = "HAPIL";
        lib_dir = format!("{}/dsolib", hfs);
    }
    println!("cargo:rustc-link-search=native={}", lib_dir);
    println!("cargo:rustc-link-lib=dylib={}", filename);
}
