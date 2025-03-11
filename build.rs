fn main() {
    // Basic Configuration
    println!("cargo:rustc-link-arg=/MERGE:.rdata=.text");
    println!("cargo:rustc-link-arg=/STACK:0x800000");

    // Security Features
    println!("cargo:rustc-link-arg=/DYNAMICBASE");
    println!("cargo:rustc-link-arg=/CETCOMPAT");
    println!("cargo:rustc-link-arg=/NXCOMPAT");
    println!("cargo:rustc-link-arg=/GUARD:CF");
    println!("cargo:rustc-link-arg=/GUARD:EHCONT");
    println!("cargo:rustc-link-arg=/FORCE:GUARDEHCONT");
    println!("cargo:rustc-link-arg=/DEPENDENTLOADFLAG:1");
    println!("cargo:rustc-link-arg=/HIGHENTROPYVA");

    // Optimization Settings
    println!("cargo:rustc-link-arg=/OPT:ICF=3");
    println!("cargo:rustc-link-arg=/OPT:REF");
    println!("cargo:rustc-link-arg=/RELEASE");
    println!("cargo:rustc-link-arg=/OPT:LBR");
    println!("cargo:rustc-link-arg=/LTCG");
    println!("cargo:rustc-link-arg=/INCREMENTAL:NO");
    println!("cargo:rustc-link-arg=/BREPRO");

    // Disable debug information
    println!("cargo:rustc-link-arg=/DEBUG:NONE");
    println!("cargo:rustc-link-arg=/NOCOFFGRPINFO");
    println!("cargo:rustc-link-arg=/PDBALTPATH:none");
}
