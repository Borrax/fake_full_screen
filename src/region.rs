pub const MIN_REGION_SIZE: f32 = 80.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Vertical,
    Horizontal,
}

/// A rectangular screen region.
///
/// Regions form a binary tree: a leaf is unsplit, an inner node owns two
/// children separated by a divider.
#[derive(Debug, Clone)]
pub enum Region {
    Leaf {
        rect: egui::Rect,
    },
    Split {
        direction: SplitDirection,
        // normalised divider position within the parent rect (0.0 – 1.0)
        ratio: f32,
        children: Box<[Region; 2]>,
    },
}

impl Region {
    pub fn new(rect: egui::Rect) -> Self {
        Region::Leaf { rect }
    }

    /// Try to split the leaf that contains `point` along `direction`.
    /// Returns true if a split was performed.
    pub fn try_split(&mut self, point: egui::Pos2, direction: SplitDirection) -> bool {
        match self {
            Region::Leaf { rect } => {
                if !rect.contains(point) {
                    return false;
                }

                let ratio = match direction {
                    SplitDirection::Vertical => (point.x - rect.left()) / rect.width(),
                    SplitDirection::Horizontal => (point.y - rect.top()) / rect.height(),
                };

                let min_ratio = match direction {
                    SplitDirection::Vertical => MIN_REGION_SIZE / rect.width(),
                    SplitDirection::Horizontal => MIN_REGION_SIZE / rect.height(),
                };
                let ratio = ratio.clamp(min_ratio, 1.0 - min_ratio);

                let (child_a, child_b) = split_rect(*rect, direction, ratio);

                if child_a.width() < MIN_REGION_SIZE
                    || child_a.height() < MIN_REGION_SIZE
                    || child_b.width() < MIN_REGION_SIZE
                    || child_b.height() < MIN_REGION_SIZE
                {
                    return false;
                }

                *self = Region::Split {
                    direction,
                    ratio,
                    children: Box::new([Region::new(child_a), Region::new(child_b)]),
                };
                true
            }
            Region::Split { children, .. } => {
                children[0].try_split(point, direction)
                    || children[1].try_split(point, direction)
            }
        }
    }

    pub fn leaves(&self) -> Vec<egui::Rect> {
        match self {
            Region::Leaf { rect } => vec![*rect],
            Region::Split { children, .. } => {
                let mut v = children[0].leaves();
                v.extend(children[1].leaves());
                v
            }
        }
    }
}

pub fn split_rect(rect: egui::Rect, dir: SplitDirection, ratio: f32) -> (egui::Rect, egui::Rect) {
    match dir {
        SplitDirection::Vertical => {
            let split_x = rect.left() + rect.width() * ratio;
            (
                egui::Rect::from_min_max(rect.min, egui::pos2(split_x, rect.max.y)),
                egui::Rect::from_min_max(egui::pos2(split_x, rect.min.y), rect.max),
            )
        }
        SplitDirection::Horizontal => {
            let split_y = rect.top() + rect.height() * ratio;
            (
                egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, split_y)),
                egui::Rect::from_min_max(egui::pos2(rect.min.x, split_y), rect.max),
            )
        }
    }
}
