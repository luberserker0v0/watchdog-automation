use std::process::Command;
use tauri::AppHandle;

#[tauri::command]
pub fn open_target_window(_app: AppHandle) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    let dir = exe.parent().ok_or("Failed to get exe parent dir")?;

    log::info!("[target] current_exe={}", exe.display());
    log::info!("[target] looking for target exe in={}", dir.display());

    let target_exe = if cfg!(target_os = "windows") {
        dir.join("watch-dog-target.exe")
    } else {
        dir.join("watch-dog-target")
    };

    log::info!("[target] target_exe={}", target_exe.display());
    log::info!("[target] exists={}", target_exe.exists());

    if !target_exe.exists() {
        return Err(format!("Target exe not found: {}", target_exe.display()));
    }

    Command::new(&target_exe)
        .spawn()
        .map_err(|e| format!("Failed to open target: {}", e))?;

    log::info!("[target] spawned successfully");
    Ok(())
}
