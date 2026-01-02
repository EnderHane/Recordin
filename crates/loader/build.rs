fn main() {
    // println!("cargo:rustc-link-lib=mfuuid");
    // println!("cargo:rustc-link-lib=strmiids");
    // println!("cargo:rustc-link-lib=mfplat");
    // println!("cargo:rustc-link-lib=User32");
    // println!("cargo:rustc-link-lib=Crypt32");
    // println!("cargo:rustc-link-lib=WS2_32");
    // println!("cargo:rustc-link-lib=Secur32");
    println!("cargo:rustc-link-lib=bcrypt");
    println!("cargo:rustc-link-lib=libx264");
    println!("cargo:rustc-link-lib=x265-static");
}
