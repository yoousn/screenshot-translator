#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CursorPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorNudgeReport {
    pub attempted: bool,
    pub ok: bool,
    pub original: Option<CursorPoint>,
    pub nudged: Option<CursorPoint>,
    pub after_nudge: Option<CursorPoint>,
    pub restored: Option<CursorPoint>,
    pub restore_attempted: bool,
    pub restore_confirmed: bool,
    pub dx: i32,
    pub dy: i32,
    pub error: Option<String>,
}

impl CursorNudgeReport {
    fn blocked(dx: i32, dy: i32, error: impl ToString) -> Self {
        Self {
            attempted: false,
            ok: false,
            original: None,
            nudged: None,
            after_nudge: None,
            restored: None,
            restore_attempted: false,
            restore_confirmed: false,
            dx,
            dy,
            error: Some(error.to_string()),
        }
    }
}

pub fn nudge_cursor_temporarily(dx: i32, dy: i32) -> CursorNudgeReport {
    if dx == 0 && dy == 0 {
        return CursorNudgeReport::blocked(dx, dy, "cursor nudge requires non-zero movement");
    }
    if dx.abs() > 2 || dy.abs() > 2 {
        return CursorNudgeReport::blocked(
            dx,
            dy,
            "cursor nudge is limited to two pixels per axis",
        );
    }

    #[cfg(not(target_os = "windows"))]
    {
        CursorNudgeReport::blocked(dx, dy, "cursor nudge requires Windows")
    }

    #[cfg(target_os = "windows")]
    {
        nudge_cursor_temporarily_windows(dx, dy)
    }
}

#[cfg(target_os = "windows")]
fn nudge_cursor_temporarily_windows(dx: i32, dy: i32) -> CursorNudgeReport {
    let original = match get_cursor_pos_windows() {
        Ok(point) => point,
        Err(error) => return CursorNudgeReport::blocked(dx, dy, error),
    };
    let nudged = CursorPoint {
        x: original.x.saturating_add(dx),
        y: original.y.saturating_add(dy),
    };
    let mut error = None;
    let mut after_nudge = None;

    if let Err(set_error) = set_cursor_pos_windows(nudged) {
        error = Some(set_error);
    } else {
        after_nudge = get_cursor_pos_windows().ok();
    }

    let restore_attempted = true;
    let restore_error = set_cursor_pos_windows(original).err();
    let restored = get_cursor_pos_windows().ok();
    let restore_confirmed = restored == Some(original);
    if let Some(restore_error) = restore_error {
        error = Some(match error {
            Some(error) => format!("{error}; restore failed: {restore_error}"),
            None => format!("restore failed: {restore_error}"),
        });
    } else if !restore_confirmed {
        error = Some(match error {
            Some(error) => format!("{error}; cursor restore was not confirmed"),
            None => "cursor restore was not confirmed".to_string(),
        });
    }

    CursorNudgeReport {
        attempted: true,
        ok: error.is_none() && restore_confirmed,
        original: Some(original),
        nudged: Some(nudged),
        after_nudge,
        restored,
        restore_attempted,
        restore_confirmed,
        dx,
        dy,
        error,
    }
}

#[cfg(target_os = "windows")]
fn get_cursor_pos_windows() -> Result<CursorPoint, String> {
    let mut point = crate::win32::POINT { x: 0, y: 0 };
    let ok = unsafe { crate::win32::GetCursorPos(&mut point) };
    if ok == 0 {
        Err("GetCursorPos failed".to_string())
    } else {
        Ok(CursorPoint {
            x: point.x,
            y: point.y,
        })
    }
}

#[cfg(target_os = "windows")]
fn set_cursor_pos_windows(point: CursorPoint) -> Result<(), String> {
    let ok = unsafe { crate::win32::SetCursorPos(point.x, point.y) };
    if ok == 0 {
        Err(format!("SetCursorPos failed for {},{}", point.x, point.y))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_nudge_rejects_zero_movement() {
        let report = nudge_cursor_temporarily(0, 0);
        assert!(!report.attempted);
        assert!(!report.ok);
        assert!(report.error.unwrap().contains("non-zero movement"));
    }

    #[test]
    fn cursor_nudge_rejects_large_movement() {
        let report = nudge_cursor_temporarily(3, 0);
        assert!(!report.attempted);
        assert!(!report.ok);
        assert!(report.error.unwrap().contains("two pixels"));
    }
}
