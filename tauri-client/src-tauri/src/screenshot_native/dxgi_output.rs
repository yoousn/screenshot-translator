use super::MonitorCaptureBounds;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DxgiDesktopCoordinates {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl DxgiDesktopCoordinates {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn bounds(self) -> Option<MonitorCaptureBounds> {
        let width = self.right.checked_sub(self.left)?;
        let height = self.bottom.checked_sub(self.top)?;
        if width <= 0 || height <= 0 {
            return None;
        }
        Some(MonitorCaptureBounds::new(
            self.left,
            self.top,
            u32::try_from(width).ok()?,
            u32::try_from(height).ok()?,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DxgiOutputCandidate {
    pub adapter_index: u32,
    pub output_index: u32,
    pub desktop_bounds: MonitorCaptureBounds,
}

impl DxgiOutputCandidate {
    pub const fn new(
        adapter_index: u32,
        output_index: u32,
        desktop_bounds: MonitorCaptureBounds,
    ) -> Self {
        Self {
            adapter_index,
            output_index,
            desktop_bounds,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DxgiOutputSelection {
    pub candidate: DxgiOutputCandidate,
    pub intersection_area: u64,
    pub contains_selection_center: bool,
}

pub const DXGI_OUTPUT_RANKING_POLICY_VERSION: u32 = 1;
pub const DXGI_OUTPUT_RANKING_POLICY: &str =
    "largest-intersection-area-then-selection-center-then-adapter-output-order";

#[derive(Debug, Clone, PartialEq)]
pub struct DxgiOutputRankingEvidence {
    pub policy_version: u32,
    pub policy: &'static str,
    pub requested_bounds: MonitorCaptureBounds,
    pub selection_center: Option<(i64, i64)>,
    pub candidate_count: usize,
    pub selected_rank: Option<u32>,
    pub selected_output: Option<DxgiOutputCandidate>,
    pub ranked_outputs: Vec<DxgiRankedOutputEvidence>,
    pub persistent_handle_exposed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DxgiRankedOutputEvidence {
    pub rank: u32,
    pub candidate: DxgiOutputCandidate,
    pub intersection_bounds: Option<MonitorCaptureBounds>,
    pub intersection_area: u64,
    pub intersection_ratio: f64,
    pub contains_selection_center: bool,
    pub selectable: bool,
    pub selected: bool,
    pub rejection_reason: Option<&'static str>,
}

pub fn select_dxgi_output_for_selection(
    selection: MonitorCaptureBounds,
    candidates: &[DxgiOutputCandidate],
) -> Option<DxgiOutputSelection> {
    rank_dxgi_outputs_for_selection(selection, candidates)
        .ranked_outputs
        .into_iter()
        .find(|output| output.selected)
        .map(|output| DxgiOutputSelection {
            candidate: output.candidate,
            intersection_area: output.intersection_area,
            contains_selection_center: output.contains_selection_center,
        })
}

pub fn rank_dxgi_outputs_for_selection(
    selection: MonitorCaptureBounds,
    candidates: &[DxgiOutputCandidate],
) -> DxgiOutputRankingEvidence {
    let selection_valid =
        !selection.is_empty() && selection.right().is_some() && selection.bottom().is_some();
    let selection_center = if selection_valid {
        Some(selection_center(selection))
    } else {
        None
    };
    let selection_area = if selection_valid {
        Some(u64::from(selection.width) * u64::from(selection.height))
    } else {
        None
    };
    let mut ranked_outputs = candidates
        .iter()
        .copied()
        .map(|candidate| ranked_output_without_rank(selection, selection_area, candidate))
        .collect::<Vec<_>>();

    ranked_outputs.sort_by(compare_ranked_output_evidence);
    let selected_index = ranked_outputs.iter().position(|output| output.selectable);
    for (index, output) in ranked_outputs.iter_mut().enumerate() {
        output.rank = (index as u32).saturating_add(1);
        output.selected = Some(index) == selected_index;
    }

    let selected_rank = selected_index.map(|index| (index as u32).saturating_add(1));
    let selected_output = selected_index.map(|index| ranked_outputs[index].candidate);
    DxgiOutputRankingEvidence {
        policy_version: DXGI_OUTPUT_RANKING_POLICY_VERSION,
        policy: DXGI_OUTPUT_RANKING_POLICY,
        requested_bounds: selection,
        selection_center,
        candidate_count: candidates.len(),
        selected_rank,
        selected_output,
        ranked_outputs,
        persistent_handle_exposed: false,
    }
}

fn compare_ranked_output_evidence(
    left: &DxgiRankedOutputEvidence,
    right: &DxgiRankedOutputEvidence,
) -> std::cmp::Ordering {
    right
        .selectable
        .cmp(&left.selectable)
        .then_with(|| right.intersection_area.cmp(&left.intersection_area))
        .then_with(|| {
            right
                .contains_selection_center
                .cmp(&left.contains_selection_center)
        })
        .then_with(|| {
            left.candidate
                .adapter_index
                .cmp(&right.candidate.adapter_index)
        })
        .then_with(|| {
            left.candidate
                .output_index
                .cmp(&right.candidate.output_index)
        })
}

fn ranked_output_without_rank(
    selection: MonitorCaptureBounds,
    selection_area: Option<u64>,
    candidate: DxgiOutputCandidate,
) -> DxgiRankedOutputEvidence {
    let intersection_bounds =
        selection_area.and_then(|_| intersection_bounds(selection, candidate.desktop_bounds));
    let intersection_area = intersection_bounds
        .map(|bounds| u64::from(bounds.width) * u64::from(bounds.height))
        .unwrap_or(0);
    let selectable = intersection_area > 0;
    DxgiRankedOutputEvidence {
        rank: 0,
        candidate,
        intersection_bounds,
        intersection_area,
        intersection_ratio: selection_area
            .filter(|area| *area > 0)
            .map(|area| intersection_area as f64 / area as f64)
            .unwrap_or(0.0),
        contains_selection_center: selectable
            && contains_selection_center(selection, candidate.desktop_bounds),
        selectable,
        selected: false,
        rejection_reason: if selectable {
            None
        } else if selection_area.is_none() {
            Some("invalid-selection")
        } else {
            Some("no-intersection")
        },
    }
}

fn selection_center(selection: MonitorCaptureBounds) -> (i64, i64) {
    (
        i64::from(selection.origin_x) + i64::from(selection.width) / 2,
        i64::from(selection.origin_y) + i64::from(selection.height) / 2,
    )
}

fn intersection_bounds(
    selection: MonitorCaptureBounds,
    output: MonitorCaptureBounds,
) -> Option<MonitorCaptureBounds> {
    let selection_right = i64::from(selection.right()?);
    let selection_bottom = i64::from(selection.bottom()?);
    let output_right = i64::from(output.right()?);
    let output_bottom = i64::from(output.bottom()?);
    let left = i64::from(selection.origin_x).max(i64::from(output.origin_x));
    let top = i64::from(selection.origin_y).max(i64::from(output.origin_y));
    let right = selection_right.min(output_right);
    let bottom = selection_bottom.min(output_bottom);
    if right <= left || bottom <= top {
        return None;
    }
    Some(MonitorCaptureBounds::new(
        i32::try_from(left).ok()?,
        i32::try_from(top).ok()?,
        u32::try_from(right - left).ok()?,
        u32::try_from(bottom - top).ok()?,
    ))
}

fn contains_selection_center(
    selection: MonitorCaptureBounds,
    output: MonitorCaptureBounds,
) -> bool {
    let Some(selection_right) = selection.right() else {
        return false;
    };
    let Some(selection_bottom) = selection.bottom() else {
        return false;
    };
    let Some(output_right) = output.right() else {
        return false;
    };
    let Some(output_bottom) = output.bottom() else {
        return false;
    };
    let center_x = i64::from(selection.origin_x) + i64::from(selection.width) / 2;
    let center_y = i64::from(selection.origin_y) + i64::from(selection.height) / 2;
    center_x >= i64::from(output.origin_x)
        && center_x < i64::from(output_right)
        && center_y >= i64::from(output.origin_y)
        && center_y < i64::from(output_bottom)
        && selection_right > output.origin_x
        && selection_bottom > output.origin_y
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        adapter_index: u32,
        output_index: u32,
        bounds: MonitorCaptureBounds,
    ) -> DxgiOutputCandidate {
        DxgiOutputCandidate::new(adapter_index, output_index, bounds)
    }

    #[test]
    fn dxgi_desktop_coordinates_convert_to_desktop_bounds() {
        assert_eq!(
            DxgiDesktopCoordinates::new(-1920, 0, 0, 1080).bounds(),
            Some(MonitorCaptureBounds::new(-1920, 0, 1920, 1080))
        );
        assert_eq!(DxgiDesktopCoordinates::new(10, 10, 10, 20).bounds(), None);
        assert_eq!(DxgiDesktopCoordinates::new(10, 10, 20, 10).bounds(), None);
        assert_eq!(
            DxgiDesktopCoordinates::new(i32::MIN, 0, i32::MAX, 10).bounds(),
            None
        );
    }

    #[test]
    fn ranks_output_by_largest_intersection_area() {
        let outputs = [
            candidate(0, 0, MonitorCaptureBounds::new(-1920, 0, 1920, 1080)),
            candidate(0, 1, MonitorCaptureBounds::new(0, 0, 1920, 1080)),
        ];

        let selected = select_dxgi_output_for_selection(
            MonitorCaptureBounds::new(-50, 100, 200, 120),
            &outputs,
        )
        .expect("ranked output");

        assert_eq!(selected.candidate.output_index, 1);
        assert_eq!(selected.intersection_area, 150 * 120);
        assert!(selected.contains_selection_center);
    }

    #[test]
    fn ranks_negative_origin_output_when_selection_is_inside_it() {
        let outputs = [
            candidate(0, 0, MonitorCaptureBounds::new(-1920, 0, 1920, 1080)),
            candidate(0, 1, MonitorCaptureBounds::new(0, 0, 1920, 1080)),
        ];

        let selected = select_dxgi_output_for_selection(
            MonitorCaptureBounds::new(-1910, 20, 300, 200),
            &outputs,
        )
        .expect("ranked output");

        assert_eq!(selected.candidate.output_index, 0);
        assert_eq!(selected.intersection_area, 300 * 200);
        assert!(selected.contains_selection_center);
    }

    #[test]
    fn breaks_equal_seam_intersection_by_selection_center() {
        let outputs = [
            candidate(0, 0, MonitorCaptureBounds::new(-1920, 0, 1920, 1080)),
            candidate(0, 1, MonitorCaptureBounds::new(0, 0, 1920, 1080)),
        ];

        let selected =
            select_dxgi_output_for_selection(MonitorCaptureBounds::new(-10, 0, 20, 100), &outputs)
                .expect("ranked output");

        assert_eq!(selected.candidate.output_index, 1);
        assert_eq!(selected.intersection_area, 10 * 100);
        assert!(selected.contains_selection_center);
    }

    #[test]
    fn breaks_identical_output_ties_by_stable_adapter_output_order() {
        let outputs = [
            candidate(0, 1, MonitorCaptureBounds::new(0, 0, 1920, 1080)),
            candidate(0, 0, MonitorCaptureBounds::new(0, 0, 1920, 1080)),
        ];

        let selected = select_dxgi_output_for_selection(
            MonitorCaptureBounds::new(100, 100, 20, 100),
            &outputs,
        )
        .expect("ranked output");

        assert_eq!(selected.candidate.output_index, 0);
        assert_eq!(selected.intersection_area, 20 * 100);
    }

    #[test]
    fn rejects_non_intersecting_outputs() {
        let outputs = [candidate(
            0,
            0,
            MonitorCaptureBounds::new(1920, 0, 1920, 1080),
        )];

        assert_eq!(
            select_dxgi_output_for_selection(
                MonitorCaptureBounds::new(-100, 100, 50, 50),
                &outputs
            ),
            None
        );
    }

    #[test]
    fn ranking_evidence_lists_all_candidates_without_handles() {
        let outputs = [
            candidate(0, 0, MonitorCaptureBounds::new(-1920, 0, 1920, 1080)),
            candidate(0, 1, MonitorCaptureBounds::new(0, 0, 1920, 1080)),
            candidate(1, 0, MonitorCaptureBounds::new(1920, 0, 1920, 1080)),
        ];

        let evidence = rank_dxgi_outputs_for_selection(
            MonitorCaptureBounds::new(-50, 100, 200, 120),
            &outputs,
        );

        assert_eq!(evidence.policy_version, DXGI_OUTPUT_RANKING_POLICY_VERSION);
        assert_eq!(evidence.policy, DXGI_OUTPUT_RANKING_POLICY);
        assert_eq!(evidence.selection_center, Some((50, 160)));
        assert_eq!(evidence.candidate_count, 3);
        assert_eq!(evidence.selected_rank, Some(1));
        assert_eq!(evidence.selected_output, Some(outputs[1]));
        assert!(!evidence.persistent_handle_exposed);
        assert_eq!(evidence.ranked_outputs.len(), 3);
        assert_eq!(evidence.ranked_outputs[0].rank, 1);
        assert_eq!(evidence.ranked_outputs[0].candidate, outputs[1]);
        assert_eq!(evidence.ranked_outputs[0].intersection_area, 150 * 120);
        assert_eq!(
            evidence.ranked_outputs[0].intersection_bounds,
            Some(MonitorCaptureBounds::new(0, 100, 150, 120))
        );
        assert!(evidence.ranked_outputs[0].selectable);
        assert!(evidence.ranked_outputs[0].selected);
        assert_eq!(
            evidence.ranked_outputs[2].rejection_reason,
            Some("no-intersection")
        );
    }

    #[test]
    fn ranking_evidence_keeps_invalid_selection_auditable() {
        let outputs = [candidate(0, 0, MonitorCaptureBounds::new(0, 0, 1920, 1080))];

        let evidence =
            rank_dxgi_outputs_for_selection(MonitorCaptureBounds::new(0, 0, 0, 120), &outputs);

        assert_eq!(evidence.selected_rank, None);
        assert_eq!(evidence.selected_output, None);
        assert_eq!(evidence.selection_center, None);
        assert_eq!(evidence.ranked_outputs.len(), 1);
        assert!(!evidence.ranked_outputs[0].selectable);
        assert_eq!(
            evidence.ranked_outputs[0].rejection_reason,
            Some("invalid-selection")
        );
    }

    #[test]
    fn ranking_evidence_preserves_area_before_center_policy() {
        let outputs = [
            candidate(0, 0, MonitorCaptureBounds::new(0, 0, 49, 100)),
            candidate(0, 1, MonitorCaptureBounds::new(50, 0, 10, 100)),
        ];

        let evidence =
            rank_dxgi_outputs_for_selection(MonitorCaptureBounds::new(0, 0, 100, 100), &outputs);

        assert_eq!(evidence.selected_output, Some(outputs[0]));
        assert_eq!(evidence.ranked_outputs[0].intersection_area, 49 * 100);
        assert!(!evidence.ranked_outputs[0].contains_selection_center);
        assert_eq!(evidence.ranked_outputs[1].intersection_area, 10 * 100);
        assert!(evidence.ranked_outputs[1].contains_selection_center);
    }
}
