pub mod traits;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

use std::sync::OnceLock;

use traits::PlatformProvider;

fn create_platform() -> Box<dyn PlatformProvider> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsPlatform::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacosPlatform)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxPlatform)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        compile_error!("Unsupported platform — add impl for your OS here");
    }
}

static PLATFORM: OnceLock<Box<dyn PlatformProvider>> = OnceLock::new();

pub fn get_platform() -> &'static Box<dyn PlatformProvider> {
    PLATFORM.get_or_init(|| {
        log::info!("[platform] initializing platform provider");
        let p = create_platform();
        log::info!("[platform] platform provider ready");
        p
    })
}
