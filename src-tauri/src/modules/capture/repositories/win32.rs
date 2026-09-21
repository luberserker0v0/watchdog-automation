#![allow(non_snake_case)]

use std::ffi::c_void;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows_core::BOOL;

pub fn get_window_title(hwnd: isize) -> String {
    unsafe {
        let mut buf = [0u16; 512];
        let len = GetWindowTextW(HWND(hwnd as *mut c_void), &mut buf);
        if len > 0 {
            String::from_utf16_lossy(&buf[..len as usize])
        } else {
            String::new()
        }
    }
}

pub fn is_window_minimized(hwnd: isize) -> bool {
    unsafe { IsIconic(HWND(hwnd as *mut c_void)).as_bool() }
}

pub fn get_class_name(hwnd: isize) -> String {
    unsafe {
        let mut buf = [0u16; 256];
        let len = GetClassNameW(HWND(hwnd as *mut c_void), &mut buf);
        if len > 0 {
            String::from_utf16_lossy(&buf[..len as usize])
        } else {
            String::new()
        }
    }
}

pub fn get_window_process_name(hwnd: isize) -> String {
    unsafe {
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(HWND(hwnd as *mut c_void), Some(&mut pid));
        if pid == 0 {
            return String::new();
        }
        let handle = windows::Win32::System::Threading::OpenProcess(
            windows::Win32::System::Threading::PROCESS_QUERY_INFORMATION
                | windows::Win32::System::Threading::PROCESS_VM_READ,
            false,
            pid,
        );
        if handle.is_err() {
            return String::new();
        }
        let handle = handle.unwrap();
        let mut name_buf = [0u16; 260];
        let len =
            windows::Win32::System::ProcessStatus::GetModuleBaseNameW(handle, None, &mut name_buf);
        let _ = CloseHandle(handle);
        if len > 0 {
            let name = String::from_utf16_lossy(&name_buf[..len as usize]);
            name.trim_end_matches(".exe").to_string()
        } else {
            String::new()
        }
    }
}

pub fn get_all_hwnds() -> Vec<isize> {
    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let hwnds = &mut *(lparam.0 as *mut Vec<isize>);
        hwnds.push(hwnd.0 as isize);
        BOOL(1)
    }

    let mut hwnds = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(enum_callback),
            LPARAM(&mut hwnds as *mut Vec<isize> as isize),
        );
    }
    hwnds
}
