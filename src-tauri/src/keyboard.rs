//! Keyboard simulation (Ctrl+C / Ctrl+V) via raw Win32 SendInput — no winapi/windows-rs
//! dependency, so the FFI structs below are declared by hand.

#[cfg(target_os = "windows")]
mod ffi {
    pub const VK_CONTROL: u16 = 0x11;
    pub const VK_C: u16 = 0x43;
    pub const VK_V: u16 = 0x56;

    pub const KEYEVENTF_KEYUP: u32 = 0x0002;
    pub const INPUT_KEYBOARD: u32 = 1;

    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct KEYBDINPUT {
        pub w_vk: u16,
        pub w_scan: u16,
        pub dw_flags: u32,
        pub time: u32,
        pub dw_extra_info: usize,
    }

    // Win32 INPUT is `DWORD type` followed by a union of MOUSEINPUT/KEYBDINPUT/HARDWAREINPUT.
    // MOUSEINPUT is the largest member, so KEYBDINPUT plus an 8-byte tail reaches the union
    // size: sizeof(INPUT) == 40 on x64, 28 on x86. Guarded by compile-time asserts below.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct INPUT {
        pub input_type: u32,
        pub ki: KEYBDINPUT,
        pub spare: [u8; 8],
    }

    #[cfg(target_pointer_width = "64")]
    const _: () = assert!(std::mem::size_of::<INPUT>() == 40);
    #[cfg(target_pointer_width = "32")]
    const _: () = assert!(std::mem::size_of::<INPUT>() == 28);

    #[link(name = "user32")]
    extern "system" {
        pub fn SendInput(c_inputs: u32, p_inputs: *const INPUT, cb_size: i32) -> u32;
    }
}

/// Simulate a Ctrl+V keypress in the foreground window.
#[cfg(target_os = "windows")]
pub fn send_ctrl_v() -> anyhow::Result<()> {
    send_with_ctrl(ffi::VK_V)
}

/// Simulate a Ctrl+C keypress in the foreground window.
#[cfg(target_os = "windows")]
pub fn send_ctrl_c() -> anyhow::Result<()> {
    send_with_ctrl(ffi::VK_C)
}

#[cfg(target_os = "windows")]
fn send_with_ctrl(key_vk: u16) -> anyhow::Result<()> {
    send_key(ffi::VK_CONTROL, false)?;
    std::thread::sleep(std::time::Duration::from_millis(30));
    send_key(key_vk, false)?;
    std::thread::sleep(std::time::Duration::from_millis(30));
    send_key(key_vk, true)?;
    std::thread::sleep(std::time::Duration::from_millis(30));
    send_key(ffi::VK_CONTROL, true)
}

#[cfg(target_os = "windows")]
fn send_key(vk: u16, key_up: bool) -> anyhow::Result<()> {
    let input = ffi::INPUT {
        input_type: ffi::INPUT_KEYBOARD,
        ki: ffi::KEYBDINPUT {
            w_vk: vk,
            w_scan: 0,
            dw_flags: if key_up { ffi::KEYEVENTF_KEYUP } else { 0 },
            time: 0,
            dw_extra_info: 0,
        },
        spare: [0; 8],
    };
    let sent = unsafe { ffi::SendInput(1, &input, std::mem::size_of::<ffi::INPUT>() as i32) };
    if sent != 1 {
        anyhow::bail!("SendInput failed for virtual key {:#04x}", vk);
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn send_ctrl_v() -> anyhow::Result<()> {
    anyhow::bail!("keyboard simulation is not supported on this platform")
}

#[cfg(not(target_os = "windows"))]
pub fn send_ctrl_c() -> anyhow::Result<()> {
    anyhow::bail!("keyboard simulation is not supported on this platform")
}
