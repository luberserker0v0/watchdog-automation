#![allow(non_snake_case)]

use super::traits::{DesktopInfo, VDError, VirtualDesktopManager};
use crate::modules::capture::repositories::win32::{get_all_hwnds, get_window_title};
use std::collections::HashSet;
use std::ffi::c_void;
use std::sync::Once;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::Com::*;
use windows_core as _;

static COM_INIT: Once = Once::new();

fn ensure_com() {
    COM_INIT.call_once(|| unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    });
}

const CLSID_VIRTUAL_DESKTOP_MANAGER: GUID = GUID::from_u128(0xAA509086_5CA9_4C25_8F95_589D3C07B48A);

#[windows::core::interface("a5cd92ff-29be-454c-8d04-d82879fb3f1b")]
unsafe trait IVirtualDesktopManager: IUnknown {
    fn IsWindowOnCurrentVirtualDesktop(
        &self,
        toplevelwindow: HWND,
        oncurrentdesktop: *mut BOOL,
    ) -> HRESULT;
    fn GetWindowDesktopId(&self, toplevelwindow: HWND, desktopid: *mut GUID) -> HRESULT;
    fn MoveWindowToDesktop(&self, toplevelwindow: HWND, desktopid: *const GUID) -> HRESULT;
}

pub struct WindowsDesktopManager {
    manager: Option<IVirtualDesktopManager>,
}

unsafe impl Send for WindowsDesktopManager {}
unsafe impl Sync for WindowsDesktopManager {}

impl WindowsDesktopManager {
    pub fn new() -> Option<Self> {
        ensure_com();

        log::info!("[vd] creating IVirtualDesktopManager");
        let manager: Result<IVirtualDesktopManager> =
            unsafe { CoCreateInstance(&CLSID_VIRTUAL_DESKTOP_MANAGER, None, CLSCTX_INPROC_SERVER) };

        match manager {
            Ok(m) => {
                log::info!("[vd] IVirtualDesktopManager created successfully");
                Some(WindowsDesktopManager { manager: Some(m) })
            }
            Err(e) => {
                log::error!(
                    "[vd] IVirtualDesktopManager CoCreateInstance failed: {:?}",
                    e
                );
                None
            }
        }
    }
}

impl VirtualDesktopManager for WindowsDesktopManager {
    fn get_all_desktops(&self) -> std::result::Result<Vec<DesktopInfo>, VDError> {
        log::info!("[vd] get_all_desktops called (EnumWindows approach)");
        let manager = self.manager.as_ref().ok_or(VDError::NotInitialized)?;

        let hwnds = get_all_hwnds();
        log::info!("[vd] EnumWindows found {} total HWNDs", hwnds.len());

        let mut seen = HashSet::new();
        let mut desktops = Vec::new();

        for &hwnd in &hwnds {
            let title = get_window_title(hwnd);

            if title == "Default IME" || title == "MSCTFIME UI" {
                continue;
            }

            let mut desktop_id = GUID::default();
            let hr =
                unsafe { manager.GetWindowDesktopId(HWND(hwnd as *mut c_void), &mut desktop_id) };

            if hr.is_err() {
                continue;
            }
            if desktop_id == GUID::default() {
                continue;
            }

            let id_str = format!("{:?}", desktop_id);
            if seen.contains(&id_str) {
                continue;
            }
            seen.insert(id_str.clone());

            let mut is_current = BOOL(0);
            let hr = unsafe {
                manager.IsWindowOnCurrentVirtualDesktop(HWND(hwnd as *mut c_void), &mut is_current)
            };
            let is_current = hr.is_ok() && is_current.as_bool();

            log::info!(
                "[vd] found desktop {} is_current={} from window '{}'",
                id_str,
                is_current,
                title
            );
            desktops.push(DesktopInfo {
                id: id_str,
                name: Some(format!("Desktop {}", desktops.len() + 1)),
                index: desktops.len() as u32,
                is_current,
            });
        }

        log::info!(
            "[vd] returning {} desktops (from {} HWNDs)",
            desktops.len(),
            hwnds.len()
        );
        Ok(desktops)
    }

    fn get_current_desktop(&self) -> std::result::Result<DesktopInfo, VDError> {
        let manager = self.manager.as_ref().ok_or(VDError::NotInitialized)?;

        let hwnds = get_all_hwnds();

        for &hwnd in &hwnds {
            let mut is_current = BOOL(0);
            let hr = unsafe {
                manager.IsWindowOnCurrentVirtualDesktop(HWND(hwnd as *mut c_void), &mut is_current)
            };
            if hr.is_ok() && is_current.as_bool() {
                let mut desktop_id = GUID::default();
                let hr = unsafe {
                    manager.GetWindowDesktopId(HWND(hwnd as *mut c_void), &mut desktop_id)
                };
                if hr.is_ok() && desktop_id != GUID::default() {
                    let id = format!("{:?}", desktop_id);
                    return Ok(DesktopInfo {
                        id,
                        name: Some("Current Desktop".into()),
                        index: 0,
                        is_current: true,
                    });
                }
            }
        }

        Err(VDError::ComError(
            "Could not determine current desktop".into(),
        ))
    }

    fn is_window_on_current_desktop(&self, hwnd: usize) -> std::result::Result<bool, VDError> {
        let manager = self.manager.as_ref().ok_or(VDError::NotInitialized)?;
        let hwnd = hwnd as isize;

        let mut result = BOOL(0);
        let hr = unsafe {
            manager.IsWindowOnCurrentVirtualDesktop(HWND(hwnd as *mut c_void), &mut result)
        };

        if hr.is_err() {
            return Err(VDError::ComError(format!(
                "IsWindowOnCurrentVirtualDesktop failed: {}",
                hr
            )));
        }

        Ok(result.as_bool())
    }

    fn get_window_desktop_id(&self, hwnd: usize) -> std::result::Result<Option<String>, VDError> {
        let manager = self.manager.as_ref().ok_or(VDError::NotInitialized)?;
        let hwnd = hwnd as isize;

        let mut desktop_id = GUID::default();
        let hr = unsafe { manager.GetWindowDesktopId(HWND(hwnd as *mut c_void), &mut desktop_id) };

        if hr.is_ok() && desktop_id != GUID::default() {
            Ok(Some(format!("{:?}", desktop_id)))
        } else {
            Ok(None)
        }
    }

    fn get_current_desktop_id(&self) -> std::result::Result<String, VDError> {
        let current = self.get_current_desktop()?;
        Ok(current.id)
    }
}
