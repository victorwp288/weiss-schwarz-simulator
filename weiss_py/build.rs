use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=OPT_LEVEL");
    println!("cargo:rerun-if-env-changed=TARGET");

    if let Ok(profile) = env::var("PROFILE") {
        println!("cargo:rustc-env=BUILD_PROFILE={profile}");
    }
    if let Ok(opt_level) = env::var("OPT_LEVEL") {
        println!("cargo:rustc-env=BUILD_OPT_LEVEL={opt_level}");
    }
    if let Ok(target) = env::var("TARGET") {
        println!("cargo:rustc-env=BUILD_TARGET={target}");
    }
}
