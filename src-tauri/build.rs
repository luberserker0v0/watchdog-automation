fn main() {
    // The lib test harness links Tauri/Wry code that imports TaskDialogIndirect,
    // but Cargo's generated harness does not carry Tauri's application manifest.
    // Delay-load Common Controls so pure unit tests can start without resolving
    // that unused UI entry point. The packaged app still receives and uses
    // Tauri's Common Controls v6 activation manifest.
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg=/DELAYLOAD:comctl32.dll");
        println!("cargo:rustc-link-lib=delayimp");
    }

    tauri_build::build()
}
