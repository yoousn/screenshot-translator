#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusRestorePolicy {
    RestorePreviousForeground,
    SuppressRestore,
    LeaveUnchanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AltTabRestoreSemantics {
    PreserveUserForeground,
    ForceSnapshotForeground,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusRestoreOutcome {
    RestoredPreviousForeground,
    PreservedAltTabForeground,
    Suppressed,
    LeftUnchanged,
    NoSnapshotTarget,
    OverlayStillForeground,
    UnsupportedPlatform,
}

impl FocusRestoreOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RestoredPreviousForeground => "restored-previous-foreground",
            Self::PreservedAltTabForeground => "preserved-alt-tab-foreground",
            Self::Suppressed => "suppressed",
            Self::LeftUnchanged => "left-unchanged",
            Self::NoSnapshotTarget => "no-snapshot-target",
            Self::OverlayStillForeground => "overlay-still-foreground",
            Self::UnsupportedPlatform => "unsupported-platform",
        }
    }

    pub const fn needs_diagnostics(self) -> bool {
        matches!(self, Self::NoSnapshotTarget | Self::OverlayStillForeground)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusSnapshot {
    pub hwnd: isize,
    pub policy: FocusRestorePolicy,
}

impl FocusSnapshot {
    pub fn empty(policy: FocusRestorePolicy) -> Self {
        Self { hwnd: 0, policy }
    }

    pub fn capture(policy: FocusRestorePolicy) -> Self {
        Self {
            hwnd: current_foreground_hwnd(),
            policy,
        }
    }

    pub fn with_hwnd(hwnd: isize, policy: FocusRestorePolicy) -> Self {
        Self { hwnd, policy }
    }

    pub fn has_target(self) -> bool {
        self.hwnd != 0
    }

    pub fn restore_after_overlay_close(
        self,
        overlay_hwnd: isize,
        semantics: AltTabRestoreSemantics,
    ) -> FocusRestoreOutcome {
        match self.policy {
            FocusRestorePolicy::SuppressRestore => FocusRestoreOutcome::Suppressed,
            FocusRestorePolicy::LeaveUnchanged => FocusRestoreOutcome::LeftUnchanged,
            FocusRestorePolicy::RestorePreviousForeground => {
                restore_snapshot_foreground(self.hwnd, overlay_hwnd, semantics)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusRestorePlan {
    pub snapshot: FocusSnapshot,
    pub overlay_hwnd: isize,
    pub alt_tab_semantics: AltTabRestoreSemantics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusRestoreDiagnostic {
    pub snapshot_hwnd: isize,
    pub overlay_hwnd: isize,
    pub outcome: FocusRestoreOutcome,
}

impl FocusRestoreDiagnostic {
    pub const fn from_plan(plan: FocusRestorePlan, outcome: FocusRestoreOutcome) -> Self {
        Self {
            snapshot_hwnd: plan.snapshot.hwnd,
            overlay_hwnd: plan.overlay_hwnd,
            outcome,
        }
    }

    pub const fn is_recovery_required(self) -> bool {
        self.outcome.needs_diagnostics()
    }
}

impl FocusRestorePlan {
    pub fn preserve_alt_tab(snapshot: FocusSnapshot, overlay_hwnd: isize) -> Self {
        Self {
            snapshot,
            overlay_hwnd,
            alt_tab_semantics: AltTabRestoreSemantics::PreserveUserForeground,
        }
    }

    pub fn force_snapshot(snapshot: FocusSnapshot, overlay_hwnd: isize) -> Self {
        Self {
            snapshot,
            overlay_hwnd,
            alt_tab_semantics: AltTabRestoreSemantics::ForceSnapshotForeground,
        }
    }

    pub fn restore(self) -> FocusRestoreOutcome {
        self.snapshot
            .restore_after_overlay_close(self.overlay_hwnd, self.alt_tab_semantics)
    }

    pub fn restore_with_diagnostic(self) -> FocusRestoreDiagnostic {
        FocusRestoreDiagnostic::from_plan(self, self.restore())
    }
}

#[cfg(target_os = "windows")]
fn current_foreground_hwnd() -> isize {
    unsafe { GetForegroundWindow() }
}

#[cfg(not(target_os = "windows"))]
fn current_foreground_hwnd() -> isize {
    0
}

#[cfg(target_os = "windows")]
fn restore_snapshot_foreground(
    snapshot_hwnd: isize,
    overlay_hwnd: isize,
    semantics: AltTabRestoreSemantics,
) -> FocusRestoreOutcome {
    if snapshot_hwnd == 0 || unsafe { IsWindow(snapshot_hwnd) } == 0 {
        return FocusRestoreOutcome::NoSnapshotTarget;
    }

    let current_hwnd = unsafe { GetForegroundWindow() };
    let user_moved_focus =
        current_hwnd != 0 && current_hwnd != overlay_hwnd && current_hwnd != snapshot_hwnd;
    if matches!(semantics, AltTabRestoreSemantics::PreserveUserForeground) && user_moved_focus {
        return FocusRestoreOutcome::PreservedAltTabForeground;
    }

    let ok = unsafe { SetForegroundWindow(snapshot_hwnd) };
    if ok == 0 {
        if overlay_hwnd != 0 && unsafe { GetForegroundWindow() } == overlay_hwnd {
            return FocusRestoreOutcome::OverlayStillForeground;
        }
        return FocusRestoreOutcome::NoSnapshotTarget;
    }
    FocusRestoreOutcome::RestoredPreviousForeground
}

#[cfg(not(target_os = "windows"))]
fn restore_snapshot_foreground(
    _snapshot_hwnd: isize,
    _overlay_hwnd: isize,
    _semantics: AltTabRestoreSemantics,
) -> FocusRestoreOutcome {
    FocusRestoreOutcome::UnsupportedPlatform
}

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> isize;
    fn SetForegroundWindow(hWnd: isize) -> i32;
    fn IsWindow(hWnd: isize) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suppressed_restore_reports_non_recovery_diagnostic() {
        let plan = FocusRestorePlan::preserve_alt_tab(
            FocusSnapshot::with_hwnd(42, FocusRestorePolicy::SuppressRestore),
            7,
        );

        let diagnostic = plan.restore_with_diagnostic();

        assert_eq!(diagnostic.snapshot_hwnd, 42);
        assert_eq!(diagnostic.overlay_hwnd, 7);
        assert_eq!(diagnostic.outcome, FocusRestoreOutcome::Suppressed);
        assert!(!diagnostic.is_recovery_required());
    }

    #[test]
    fn failed_restore_outcomes_request_diagnostics() {
        assert!(FocusRestoreOutcome::NoSnapshotTarget.needs_diagnostics());
        assert!(FocusRestoreOutcome::OverlayStillForeground.needs_diagnostics());
        assert!(!FocusRestoreOutcome::PreservedAltTabForeground.needs_diagnostics());
    }
}
