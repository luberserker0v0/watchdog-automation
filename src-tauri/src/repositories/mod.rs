pub mod platform;
pub mod screen;
pub mod virtual_desktop;
pub mod window;

#[cfg(target_os = "windows")]
pub mod win32;
