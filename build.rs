use cc::Build;
use pkg_config::Config;
use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=extern/ui.cpp");
    println!("cargo:rerun-if-changed=extern/tvision");

    Command::new("cmake")
        .args(&[
            "extern/tvision",
            "-B",
            "extern/tvision/build",
            "-DCMAKE_C_FLAGS=-fPIC",
            "-DCMAKE_CXX_FLAGS=-fPIC",
            "-DCMAKE_BUILD_TYPE=RelWithDebInfo",
            "-DTV_BUILD_EXAMPLES=off",
        ])
        .status()
        .unwrap();

    let jobs = env::var("NUM_JOBS").unwrap();

    Command::new("cmake")
        .args(&["--build", "extern/tvision/build", "--parallel", &jobs])
        .status()
        .unwrap();

    println!("cargo:rustc-link-search=extern/tvision/build");

    Build::new()
        .cpp(true)
        .file("extern/ui.cpp")
        .flag("-Wno-unknown-pragmas")
        .flag("-Wno-reorder")
        .include("extern/tvision/include")
        .compile("libui.a");

    println!("cargo:rustc-link-lib=tvision");
    println!("cargo:rustc-link-lib=gpm");
    Config::new()
        .atleast_version("5.9")
        .statik(true)
        .probe("ncurses")
        .unwrap();
}
