// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(feature = "app-target")]
    watch_dog_lib::run_target();

    #[cfg(all(not(feature = "app-target"), feature = "app-tester"))]
    watch_dog_lib::run_tester();

    #[cfg(not(any(feature = "app-target", feature = "app-tester")))]
    watch_dog_lib::run();
}
