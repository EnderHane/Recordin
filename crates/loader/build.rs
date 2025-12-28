fn main() {
    vcpkg::find_package("openssl").unwrap();
    vcpkg::find_package("opus").unwrap();
    vcpkg::find_package("libwebp").unwrap();
    println!("cargo:rustc-link-lib=mfuuid");
    println!("cargo:rustc-link-lib=strmiids");
    println!("cargo:rustc-link-lib=mfplat");
    println!("cargo:rustc-link-lib=User32");
    println!("cargo:rustc-link-lib=Crypt32");
    // println!("cargo:rustc-link-lib=WS2_32");
    // println!("cargo:rustc-link-lib=Secur32");
    println!("cargo:rustc-link-lib=bcrypt");
}
