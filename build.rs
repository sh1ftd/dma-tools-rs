#[cfg(feature = "branding")]
use std::path::Path;

/// Required files when branding feature is enabled
/// Also used for rerun-if-changed directives
const BRANDING_REQUIRED_FILES: &[&str] = &["src/branding/brand.webp", "src/branding/mod.rs"];

fn main() {
    // Re-run build script if branding files or feature changes
    for file in BRANDING_REQUIRED_FILES {
        println!("cargo:rerun-if-changed={file}");
    }
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_BRANDING");

    #[cfg(feature = "branding")]
    check_branding_files();

    configure_linker();
}

/// Configure linker settings for Windows builds
fn configure_linker() {
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

/// Check that all required branding files exist when branding feature is enabled
#[cfg(feature = "branding")]
fn check_branding_files() {
    let missing_files: Vec<&str> = BRANDING_REQUIRED_FILES
        .iter()
        .filter(|file| !Path::new(file).exists())
        .copied()
        .collect();

    if !missing_files.is_empty() {
        panic!(
            "\n\n=== BRANDING FEATURE ERROR ===\n\
            The 'branding' feature is enabled, but required files are missing:\n\n\
            {}\n\n\
            Either:\n\
            1. Add the missing files to enable branding, or\n\
            2. Disable the branding feature by removing '--features branding' from the build command\n\n\
            ==============================\n",
            missing_files
                .iter()
                .map(|f| format!("  - {f}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    println!("cargo:warning=Branding feature enabled - all required files found");
}
