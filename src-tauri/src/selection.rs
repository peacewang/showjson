#[cfg(target_os = "macos")]
pub fn simulate_copy() -> Result<(), String> {
    use core_graphics::{
        event::{CGEvent, CGEventFlags, CGEventTapLocation},
        event_source::{CGEventSource, CGEventSourceStateID},
    };

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> u8;
    }

    // macOS virtual key code for the ANSI C key.
    const KEY_C: u16 = 0x08;

    let trusted = unsafe { AXIsProcessTrusted() != 0 };
    if !trusted {
        return Err(
            "未获得 macOS“辅助功能”权限，无法自动复制选中文本。请在系统设置 → 隐私与安全性 → 辅助功能中允许 ShowJSON。"
                .to_string(),
        );
    }

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "无法创建 macOS 键盘事件源".to_string())?;
    let key_down = CGEvent::new_keyboard_event(source.clone(), KEY_C, true)
        .map_err(|_| "无法创建复制按键事件".to_string())?;
    let key_up = CGEvent::new_keyboard_event(source, KEY_C, false)
        .map_err(|_| "无法创建复制按键事件".to_string())?;

    key_down.set_flags(CGEventFlags::CGEventFlagCommand);
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);
    key_down.post(CGEventTapLocation::HID);
    key_up.post(CGEventTapLocation::HID);
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn simulate_copy() -> Result<(), String> {
    use std::mem::size_of;

    const INPUT_KEYBOARD: u32 = 1;
    const KEYEVENTF_KEYUP: u32 = 0x0002;
    const VK_CONTROL: u16 = 0x11;
    const VK_C: u16 = 0x43;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct MouseInput {
        dx: i32,
        dy: i32,
        mouse_data: u32,
        flags: u32,
        time: u32,
        extra_info: usize,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct KeyboardInput {
        virtual_key: u16,
        scan_code: u16,
        flags: u32,
        time: u32,
        extra_info: usize,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct HardwareInput {
        message: u32,
        parameter_low: u16,
        parameter_high: u16,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    union InputData {
        mouse: MouseInput,
        keyboard: KeyboardInput,
        hardware: HardwareInput,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Input {
        input_type: u32,
        data: InputData,
    }

    #[link(name = "user32")]
    extern "system" {
        fn SendInput(input_count: u32, inputs: *const Input, input_size: i32) -> u32;
    }

    fn keyboard_input(virtual_key: u16, key_up: bool) -> Input {
        Input {
            input_type: INPUT_KEYBOARD,
            data: InputData {
                keyboard: KeyboardInput {
                    virtual_key,
                    scan_code: 0,
                    flags: if key_up { KEYEVENTF_KEYUP } else { 0 },
                    time: 0,
                    extra_info: 0,
                },
            },
        }
    }

    let inputs = [
        keyboard_input(VK_CONTROL, false),
        keyboard_input(VK_C, false),
        keyboard_input(VK_C, true),
        keyboard_input(VK_CONTROL, true),
    ];
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            size_of::<Input>() as i32,
        )
    };
    if sent != inputs.len() as u32 {
        return Err(
            "Windows 无法向当前应用发送复制快捷键；管理员权限窗口可能会阻止该操作。".to_string(),
        );
    }
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn simulate_copy() -> Result<(), String> {
    Err("当前平台不支持自动复制选中文本".to_string())
}
