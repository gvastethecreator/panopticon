fn main() {
    println!("cargo:rerun-if-changed=ui/main.slint");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/ui-icons");
    for font_path in [
        "assets/fonts/MirandaSans-Regular.ttf",
        "assets/fonts/MirandaSans-Medium.ttf",
        "assets/fonts/MirandaSans-SemiBold.ttf",
        "assets/fonts/MirandaSans-Bold.ttf",
        "assets/fonts/MirandaSans-Italic.ttf",
        "assets/fonts/MirandaSans-MediumItalic.ttf",
        "assets/fonts/MirandaSans-SemiBoldItalic.ttf",
        "assets/fonts/MirandaSans-BoldItalic.ttf",
    ] {
        println!("cargo:rerun-if-changed={font_path}");
    }

    slint_build::compile("ui/main.slint").expect("Slint UI compilation failed");

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
