use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct TextSourceElement {
    pub text: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[cfg(target_os = "windows")]
mod platform {
    use super::TextSourceElement;
    use std::time::{Duration, Instant};
    use windows::core::BSTR;
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, TreeScope_Subtree,
    };

    fn bstr_to_string(value: BSTR) -> String {
        String::from_utf16_lossy(value.as_wide())
            .replace('\u{00a0}', " ")
            .replace('\r', "\n")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ")
            .replace(char::from(0), "")
            .trim()
            .to_string()
    }

    fn rect_to_element(text: String, rect: RECT) -> Option<TextSourceElement> {
        let x = rect.left;
        let y = rect.top;
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        if text.len() < 2 || w < 3 || h < 3 {
            return None;
        }
        Some(TextSourceElement { text, x, y, w, h })
    }

    fn is_probably_noise(text: &str) -> bool {
        let trimmed = text.trim();
        trimmed.is_empty()
            || trimmed.len() > 500
            || matches!(trimmed, "•" | "●" | "○" | "■" | "□" | "×" | "+" | "-")
    }

    pub fn collect_text_elements(
        hwnd: isize,
        max_items: usize,
        deadline_ms: u64,
    ) -> Result<Vec<TextSourceElement>, String> {
        if hwnd == 0 {
            return Ok(Vec::new());
        }

        let deadline = Duration::from_millis(deadline_ms.max(10));
        let started = Instant::now();
        let com_initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok() };
        let result = (|| -> Result<Vec<TextSourceElement>, String> {
            let automation: IUIAutomation = unsafe {
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                    .map_err(|error| format!("create UIAutomation failed: {error}"))?
            };
            let root = unsafe {
                automation
                    .ElementFromHandle(HWND(hwnd as *mut core::ffi::c_void))
                    .map_err(|error| format!("UIAutomation ElementFromHandle failed: {error}"))?
            };
            let condition = unsafe {
                automation
                    .CreateTrueCondition()
                    .map_err(|error| format!("UIAutomation condition failed: {error}"))?
            };
            let elements = unsafe {
                root.FindAll(TreeScope_Subtree, &condition)
                    .map_err(|error| format!("UIAutomation FindAll failed: {error}"))?
            };
            let length = unsafe { elements.Length().unwrap_or(0) };
            let mut items = Vec::new();
            let mut seen = std::collections::HashSet::<String>::new();

            for index in 0..length {
                if items.len() >= max_items || started.elapsed() > deadline {
                    break;
                }
                let Ok(element) = (unsafe { elements.GetElement(index) }) else {
                    continue;
                };
                let name = unsafe { element.CurrentName() }
                    .map(bstr_to_string)
                    .unwrap_or_default();
                if is_probably_noise(&name) {
                    continue;
                }
                let Ok(rect) = (unsafe { element.CurrentBoundingRectangle() }) else {
                    continue;
                };
                let Some(item) = rect_to_element(name, rect) else {
                    continue;
                };
                let key = format!("{}|{}|{}|{}|{}", item.text, item.x, item.y, item.w, item.h);
                if seen.insert(key) {
                    items.push(item);
                }
            }
            Ok(items)
        })();
        if com_initialized {
            unsafe { CoUninitialize() };
        }
        result
    }
}

#[cfg(target_os = "windows")]
pub fn collect_text_elements(
    hwnd: isize,
    max_items: usize,
    deadline_ms: u64,
) -> Result<Vec<TextSourceElement>, String> {
    platform::collect_text_elements(hwnd, max_items, deadline_ms)
}

#[cfg(not(target_os = "windows"))]
pub fn collect_text_elements(
    _hwnd: isize,
    _max_items: usize,
    _deadline_ms: u64,
) -> Result<Vec<TextSourceElement>, String> {
    Ok(Vec::new())
}
