#![allow(non_snake_case)]

use super::traits::{DesktopInfo, VDError, VirtualDesktopManager};
use crate::repositories::win32::{get_all_hwnds, get_window_title};
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

// Internal COM interfaces for desktop switching (Windows 11 22H2+)
// These are undocumented and version-specific

#[windows::core::interface("3f07f4be-50a5-42af-b133-25c4bb7c23a0")]
unsafe trait IVirtualDesktop: IUnknown {
    fn IsViewVisible(&self, view: *const IUnknown, result: *mut BOOL) -> HRESULT;
    fn GetID(&self, gid: *mut GUID) -> HRESULT;
    fn GetName(&self, name: *mut HSTRING) -> HRESULT;
    fn GetWallpaperPath(&self, path: *mut HSTRING) -> HRESULT;
    fn IsDesktopVisible(&self, result: *mut BOOL) -> HRESULT;
}

const CLSID_VDI_INTERNAL: GUID = GUID::from_u128(0xC179334C_4295_40B3_B2DD_B80C6938BEE3);

#[windows::core::interface("a90ce396-60c8-493e-905c-6699b18fd4af")]
unsafe trait IVirtualDesktopManagerInternal: IUnknown {
    fn MoveViewToDesktop(&self, view: *const IUnknown, desktop: *const IVirtualDesktop) -> HRESULT;
    fn IsViewOnCurrentVirtualDesktop(&self, view: *const IUnknown, result: *mut BOOL) -> HRESULT;
    fn IsViewOnDesktop(
        &self,
        view: *const IUnknown,
        desktop: *const IVirtualDesktop,
        result: *mut BOOL,
    ) -> HRESULT;
    fn GetAdjacentDesktop(
        &self,
        desktop: *const IVirtualDesktop,
        direction: u32,
        adjacentdesktop: *mut *mut IVirtualDesktop,
    ) -> HRESULT;
    fn SwitchDesktop(&self, desktop: *const IVirtualDesktop) -> HRESULT;
    fn GetDesktops(&self, desktops: *mut *mut IUnknown) -> HRESULT;
    fn GetCurrentDesktop(&self, desktop: *mut *mut IVirtualDesktop) -> HRESULT;
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

    fn switch_to_desktop(&self, desktop_id: &str) -> std::result::Result<(), VDError> {
        log::info!("[vd] switch_to_desktop: target={}", desktop_id);

        // Get internal manager
        let internal: Result<IVirtualDesktopManagerInternal> =
            unsafe { CoCreateInstance(&CLSID_VDI_INTERNAL, None, CLSCTX_INPROC_SERVER) };
        let internal = internal.map_err(|e| {
            log::error!(
                "[vd] Failed to create IVirtualDesktopManagerInternal: {:?}",
                e
            );
            VDError::ComError(format!(
                "IVirtualDesktopManagerInternal not available (Windows 11 22H2+ required): {}",
                e
            ))
        })?;

        // Get current desktop
        let mut current_desktop: *mut IVirtualDesktop = std::ptr::null_mut();
        unsafe {
            let _ = internal.GetCurrentDesktop(&mut current_desktop);
        }
        if current_desktop.is_null() {
            return Err(VDError::ComError("Failed to get current desktop".into()));
        }

        // Check if already on target desktop
        let current_id = unsafe {
            let mut gid = GUID::default();
            let _ = (*current_desktop).GetID(&mut gid);
            format!("{:?}", gid)
        };
        if current_id == desktop_id {
            log::info!("[vd] already on target desktop, skipping switch");
            return Ok(());
        }

        // Get all desktops and find the target
        let mut desktops_arr: *mut IUnknown = std::ptr::null_mut();
        unsafe {
            let _ = internal.GetDesktops(&mut desktops_arr);
        }
        if desktops_arr.is_null() {
            return Err(VDError::ComError("Failed to get desktops".into()));
        }

        // Get count from IObjectArray (GetCount at vtable index 3)
        let count: u32 = unsafe {
            let vtbl = *(desktops_arr as *const *const usize);
            let get_count: unsafe extern "system" fn(*const IUnknown, *mut u32) -> HRESULT =
                std::mem::transmute(*vtbl.add(3));
            let mut count = 0u32;
            let _ = get_count(desktops_arr, &mut count);
            count
        };

        let mut target_desktop: Option<*mut IVirtualDesktop> = None;

        for i in 0..count {
            // GetAt from IObjectArray (vtable index 4)
            let mut desktop_ptr: *mut IVirtualDesktop = std::ptr::null_mut();
            unsafe {
                let vtbl = *(desktops_arr as *const *const usize);
                let get_at: unsafe extern "system" fn(
                    *const IUnknown,
                    u32,
                    *const GUID,
                    *mut *mut c_void,
                ) -> HRESULT = std::mem::transmute(*vtbl.add(4));
                let iid = IVirtualDesktop::IID;
                let _ = get_at(
                    desktops_arr,
                    i,
                    &iid,
                    &mut desktop_ptr as *mut _ as *mut *mut c_void,
                );
            }

            if !desktop_ptr.is_null() {
                let id = unsafe {
                    let mut gid = GUID::default();
                    let _ = (*desktop_ptr).GetID(&mut gid);
                    format!("{:?}", gid)
                };

                if id == desktop_id {
                    target_desktop = Some(desktop_ptr);
                    break;
                }
            }
        }

        match target_desktop {
            Some(target) => {
                log::info!("[vd] found target desktop, switching...");
                let hr = unsafe { internal.SwitchDesktop(target) };

                if hr.is_ok() {
                    log::info!("[vd] switch_to_desktop succeeded");
                    Ok(())
                } else {
                    log::error!("[vd] SwitchDesktop failed: {:?}", hr);
                    Err(VDError::ComError(format!("SwitchDesktop failed: {}", hr)))
                }
            }
            None => {
                log::error!("[vd] target desktop '{}' not found", desktop_id);
                Err(VDError::ComError(format!(
                    "Desktop '{}' not found",
                    desktop_id
                )))
            }
        }
    }
}
