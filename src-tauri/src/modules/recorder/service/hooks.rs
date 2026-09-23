#[cfg(target_os = "windows")]
pub(super) mod windows_hooks {
    use super::super::RawInput;
    use lazy_static::lazy_static;
    use std::sync::{mpsc, Mutex};
    use std::time::Instant;
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::*;

    lazy_static! {
        static ref INPUT_SENDER: Mutex<Option<mpsc::Sender<RawInput>>> = Mutex::new(None);
    }

    pub fn run(sender: mpsc::Sender<RawInput>, ready: mpsc::Sender<Result<u32, String>>) {
        *INPUT_SENDER
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(sender);
        unsafe {
            let mouse = match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), None, 0) {
                Ok(hook) => hook,
                Err(error) => {
                    let _ = ready.send(Err(format!(
                        "Could not install mouse recording hook: {error}"
                    )));
                    INPUT_SENDER
                        .lock()
                        .unwrap_or_else(|poison| poison.into_inner())
                        .take();
                    return;
                }
            };
            let keyboard = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), None, 0) {
                Ok(hook) => hook,
                Err(error) => {
                    let _ = UnhookWindowsHookEx(mouse);
                    let _ = ready.send(Err(format!(
                        "Could not install keyboard recording hook: {error}"
                    )));
                    INPUT_SENDER
                        .lock()
                        .unwrap_or_else(|poison| poison.into_inner())
                        .take();
                    return;
                }
            };
            let thread_id = GetCurrentThreadId();
            let _ = ready.send(Ok(thread_id));
            let mut message = MSG::default();
            while GetMessageW(&mut message, None, 0, 0).0 > 0 {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            let _ = UnhookWindowsHookEx(mouse);
            let _ = UnhookWindowsHookEx(keyboard);
            INPUT_SENDER
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .take();
        }
    }

    pub fn stop(thread_id: u32) {
        unsafe {
            let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
    }

    unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            if data.flags & LLMHF_INJECTED == 0 {
                let at = Instant::now();
                let event = match wparam.0 as u32 {
                    WM_LBUTTONDOWN => Some(RawInput::MouseDown {
                        x: data.pt.x,
                        y: data.pt.y,
                        at,
                    }),
                    WM_LBUTTONUP => Some(RawInput::MouseUp {
                        x: data.pt.x,
                        y: data.pt.y,
                        at,
                    }),
                    WM_MOUSEWHEEL => Some(RawInput::Wheel {
                        x: data.pt.x,
                        y: data.pt.y,
                        delta: ((data.mouseData >> 16) as i16) as i32,
                        at,
                    }),
                    _ => None,
                };
                if let Some(event) = event {
                    if let Some(sender) = INPUT_SENDER
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .as_ref()
                    {
                        let _ = sender.send(event);
                    }
                }
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }

    unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code >= 0 {
            let data = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
            if !data.flags.contains(LLKHF_INJECTED) {
                let event = match wparam.0 as u32 {
                    WM_KEYDOWN | WM_SYSKEYDOWN => Some(RawInput::KeyDown {
                        code: data.vkCode,
                        at: Instant::now(),
                    }),
                    WM_KEYUP | WM_SYSKEYUP => Some(RawInput::KeyUp { code: data.vkCode }),
                    _ => None,
                };
                if let Some(event) = event {
                    if let Some(sender) = INPUT_SENDER
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .as_ref()
                    {
                        let _ = sender.send(event);
                    }
                }
            }
        }
        CallNextHookEx(None, code, wparam, lparam)
    }
}
