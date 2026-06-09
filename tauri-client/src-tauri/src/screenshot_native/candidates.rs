use super::output::SelectionRect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    Window,
    Taskbar,
    Monitor,
    Visual,
    UiAutomation,
}

impl CandidateKind {
    pub const fn default_priority(self) -> u8 {
        match self {
            Self::Window => 30,
            Self::Taskbar => 20,
            Self::Monitor => 10,
            Self::Visual => 40,
            Self::UiAutomation => 50,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateSource {
    Win32Window,
    ShellTaskbar,
    MonitorTopology,
    VisualTree,
    UiAutomation,
    Heuristic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateRefreshReason {
    SessionStarted,
    PointerMoved,
    FocusChanged,
    DisplayChanged,
    ManualRetry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateConfidence {
    Low,
    Medium,
    High,
    Exact,
}

impl CandidateConfidence {
    pub const fn score(self) -> u8 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Exact => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionCandidate {
    pub kind: CandidateKind,
    pub rect: SelectionRect,
    pub priority: u8,
}

impl SelectionCandidate {
    pub fn new(kind: CandidateKind, rect: SelectionRect) -> Self {
        Self {
            kind,
            rect,
            priority: kind.default_priority(),
        }
    }

    pub const fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn is_valid(&self) -> bool {
        self.rect.normalized().is_valid()
    }

    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        let rect = self.rect.normalized();
        rect.is_valid()
            && i64::from(x) >= i64::from(rect.x)
            && i64::from(y) >= i64::from(rect.y)
            && i64::from(x) < rect.right()
            && i64::from(y) < rect.bottom()
    }

    pub fn area(&self) -> i64 {
        let rect = self.rect.normalized();
        if !rect.is_valid() {
            return 0;
        }
        i64::from(rect.width) * i64::from(rect.height)
    }

    pub fn rank_key(&self) -> CandidateRankKey {
        CandidateRankKey {
            priority: self.priority,
            kind_weight: kind_weight(self.kind),
            smaller_area: i64::MAX.saturating_sub(self.area()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateDescriptor {
    pub candidate: SelectionCandidate,
    pub source: CandidateSource,
    pub confidence: CandidateConfidence,
    pub title: Option<String>,
}

impl CandidateDescriptor {
    pub fn new(
        candidate: SelectionCandidate,
        source: CandidateSource,
        confidence: CandidateConfidence,
    ) -> Self {
        Self {
            candidate,
            source,
            confidence,
            title: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn rank_key(&self) -> CandidateRankKey {
        let base = self.candidate.rank_key();
        CandidateRankKey {
            priority: base.priority.saturating_add(self.confidence.score()),
            kind_weight: base.kind_weight,
            smaller_area: base.smaller_area,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CandidateRankKey {
    pub priority: u8,
    pub kind_weight: u8,
    pub smaller_area: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateHit {
    pub descriptor: CandidateDescriptor,
    pub pointer_x: i32,
    pub pointer_y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CandidateSet {
    pub items: Vec<CandidateDescriptor>,
}

impl CandidateSet {
    pub fn new(items: Vec<CandidateDescriptor>) -> Self {
        Self { items }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn best_at(&self, x: i32, y: i32) -> Option<CandidateHit> {
        self.items
            .iter()
            .filter(|item| item.candidate.contains_point(x, y))
            .max_by_key(|item| item.rank_key())
            .cloned()
            .map(|descriptor| CandidateHit {
                descriptor,
                pointer_x: x,
                pointer_y: y,
            })
    }

    pub fn sorted(mut self) -> Self {
        self.items.sort_by_key(|item| item.rank_key());
        self.items.reverse();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRefreshContract {
    pub reason: CandidateRefreshReason,
    pub include_windows: bool,
    pub include_monitors: bool,
    pub include_visual_tree: bool,
    pub include_ui_automation: bool,
}

impl CandidateRefreshContract {
    pub const fn for_reason(reason: CandidateRefreshReason) -> Self {
        Self {
            reason,
            include_windows: true,
            include_monitors: true,
            include_visual_tree: true,
            include_ui_automation: true,
        }
    }
}

const fn kind_weight(kind: CandidateKind) -> u8 {
    match kind {
        CandidateKind::UiAutomation => 5,
        CandidateKind::Visual => 4,
        CandidateKind::Window => 3,
        CandidateKind::Taskbar => 2,
        CandidateKind::Monitor => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(
        kind: CandidateKind,
        rect: SelectionRect,
        source: CandidateSource,
        confidence: CandidateConfidence,
    ) -> CandidateDescriptor {
        CandidateDescriptor::new(SelectionCandidate::new(kind, rect), source, confidence)
    }

    #[test]
    fn negative_monitor_coordinates_select_containing_window() {
        let candidates = CandidateSet::new(vec![
            descriptor(
                CandidateKind::Monitor,
                SelectionRect::new(-1920, 0, 1920, 1080),
                CandidateSource::MonitorTopology,
                CandidateConfidence::Exact,
            ),
            descriptor(
                CandidateKind::Monitor,
                SelectionRect::new(0, 0, 2560, 1440),
                CandidateSource::MonitorTopology,
                CandidateConfidence::Exact,
            ),
            descriptor(
                CandidateKind::Window,
                SelectionRect::new(-1640, 120, 1280, 720),
                CandidateSource::Win32Window,
                CandidateConfidence::High,
            )
            .with_title("left monitor window"),
        ]);

        let hit = candidates.best_at(-1000, 300).expect("window hit");

        assert_eq!(hit.descriptor.candidate.kind, CandidateKind::Window);
        assert_eq!(hit.descriptor.source, CandidateSource::Win32Window);
        assert_eq!(hit.pointer_x, -1000);
    }

    #[test]
    fn dpi_scaled_monitor_edges_are_half_open() {
        let candidates = CandidateSet::new(vec![
            descriptor(
                CandidateKind::Window,
                SelectionRect::new(0, 0, 1920, 1080),
                CandidateSource::Win32Window,
                CandidateConfidence::High,
            ),
            descriptor(
                CandidateKind::Window,
                SelectionRect::new(1920, 0, 1536, 864),
                CandidateSource::Win32Window,
                CandidateConfidence::High,
            ),
        ]);

        assert_eq!(
            candidates
                .best_at(1919, 500)
                .unwrap()
                .descriptor
                .candidate
                .rect,
            SelectionRect::new(0, 0, 1920, 1080)
        );
        assert_eq!(
            candidates
                .best_at(1920, 500)
                .unwrap()
                .descriptor
                .candidate
                .rect,
            SelectionRect::new(1920, 0, 1536, 864)
        );
        assert!(candidates.best_at(3456, 500).is_none());
    }

    #[test]
    fn maximized_window_and_taskbar_edge_do_not_steal_each_other() {
        let candidates = CandidateSet::new(vec![
            descriptor(
                CandidateKind::Window,
                SelectionRect::new(0, 0, 1920, 1040),
                CandidateSource::Win32Window,
                CandidateConfidence::Exact,
            ),
            descriptor(
                CandidateKind::Taskbar,
                SelectionRect::new(0, 1040, 1920, 40),
                CandidateSource::ShellTaskbar,
                CandidateConfidence::Exact,
            ),
            descriptor(
                CandidateKind::Monitor,
                SelectionRect::new(0, 0, 1920, 1080),
                CandidateSource::MonitorTopology,
                CandidateConfidence::Exact,
            ),
        ]);

        assert_eq!(
            candidates
                .best_at(960, 1039)
                .unwrap()
                .descriptor
                .candidate
                .kind,
            CandidateKind::Window
        );
        assert_eq!(
            candidates
                .best_at(960, 1040)
                .unwrap()
                .descriptor
                .candidate
                .kind,
            CandidateKind::Taskbar
        );
    }
}
