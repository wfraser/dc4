#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    winres::WindowsResource::new()
        .set("LegalCopyright", concat!("Copyright ", env!("CARGO_PKG_AUTHORS")))
        .set("OriginalFilename", "dc4.exe")
        .set_language(0x0409) // US English
        .compile()
        .unwrap_or_else(|e| {
            eprintln!("Cargo build script failed: {}", e);
            ::std::process::exit(1);
        });
}

#[cfg(not(windows))]
fn main() {}
