fn main() {
    slint_build::compile("ui/main.slint").unwrap();

    // Embed the application icon and version metadata into the Windows
    // executable so that Explorer, the taskbar, and Alt-Tab show the correct
    // icon.  `winres` is a no-op on non-Windows targets, so this is safe for
    // Linux CI jobs (cargo doc / cargo audit).
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    if let Err(e) = res.compile() {
        // Non-fatal: print a cargo warning and continue.
        println!("cargo:warning=icon embedding failed: {e}");
    }
}
