// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/

//! This module contains extra searching functionality for `frames::Frame`.

// -------------------------------------------------------------------------------------------------

use qualia::{Direction, Position, SurfaceId};
use frame::{Frame, Geometry, Mode};

// -------------------------------------------------------------------------------------------------

/// Extension trait for `Frame` adding more search functionality.
pub trait Searching {
    /// Finds first frame suitable for building.
    /// Returns `self` if `self` has no surface ID set, its parent otherwise.
    fn find_buildable(&self) -> Option<Frame>;

    /// Finds first trunk which is `Special`.
    /// For normal frame this should be workspace.
    fn find_top(&self) -> Option<Frame>;

    /// Finds frame with given surface ID.
    fn find_with_sid(&self, sid: SurfaceId) -> Option<Frame>;

    /// Finds leaf frame contained in frame `self` containing `point` or the closest one if `point`
    /// lies outside `self`.
    fn find_pointed(&self, point: Position) -> Frame;

    /// Finds top-most frame bordering with frame `self` in given direction.
    fn find_contiguous(&self, direction: Direction, distance: u32) -> Option<Frame>;

    /// Find find bottom-most frame bordering with frame `self` in given direction.
    fn find_adjacent(&self, direction: Direction, distance: u32) -> Option<Frame>;
}

// -------------------------------------------------------------------------------------------------

impl Searching for Frame {
    fn find_buildable(&self) -> Option<Frame> {
        if self.get_sid().is_valid() {
            self.get_parent()
        } else {
            Some(self.clone())
        }
    }

    fn find_top(&self) -> Option<Frame> {
        let mut current = Some(self.clone());
        loop {
            current = if let Some(ref frame) = current {
                if frame.get_mode().is_top() {
                    return current.clone();
                }
                frame.get_parent()
            } else {
                return None;
            }
        }
    }

    fn find_with_sid(&self, sid: SurfaceId) -> Option<Frame> {
        if self.get_sid() == sid {
            Some(self.clone())
        } else {
            for subsurface in self.time_iter() {
                let result = subsurface.find_with_sid(sid);
                if result.is_some() {
                    return result;
                }
            }
            None
        }
    }

    fn find_pointed(&self, mut point: Position) -> Frame {
        point = point.casted(&self.get_area());

        for ref frame in self.time_iter() {
            if frame.get_area().contains(&point) {
                return if self.get_mode() == Mode::Leaf {
                           frame.clone()
                       } else {
                           frame.find_pointed(point)
                       };
            }
        }
        self.clone()
    }

    fn find_contiguous(&self, direction: Direction, distance: u32) -> Option<Frame> {
        // If distance is zero, this is the last step of recurrence
        if distance == 0 {
            return Some(self.clone());
        }

        if let Some(parent) = self.get_parent() {
            // Find new frame which is farther
            let mut frame = if parent.get_geometry() == Geometry::Vertical {
                if direction == Direction::North {
                    self.get_prev_space()
                } else if direction == Direction::South {
                    self.get_next_space()
                } else {
                    None
                }
            } else if parent.get_geometry() == Geometry::Horizontal {
                if direction == Direction::West {
                    self.get_prev_space()
                } else if direction == Direction::East {
                    self.get_next_space()
                } else {
                    None
                }
            } else {
                None
            };

            // If there is nothing farther go higher. If it is, decrease distance.
            let new_distance = if frame.is_some() || (direction == Direction::Up) {
                distance - 1
            } else {
                distance
            };
            if frame.is_none() {
                frame = self.get_parent();
            }

            if let Some(frame) = frame {
                // Next recurrence step if possible.
                if frame.get_mode().is_top() {
                    None
                } else {
                    frame.find_contiguous(direction, new_distance)
                }
            } else {
                None
            }
        } else {
            // There is nowhere to go if nothing is up there
            None
        }
    }

    fn find_adjacent(&self, direction: Direction, distance: u32) -> Option<Frame> {
        // Calculate reference position
        let point = self.get_area().calculate_center();

        // Search for the frame
        let mut frame = Some(self.clone());
        for _ in 0..distance {
            frame = if let Some(ref frame) = frame {
                frame.find_contiguous(direction, 1)
            } else {
                break;
            };
            frame = if let Some(ref frame) = frame {
                Some(frame.find_pointed(point.clone()))
            } else {
                break;
            };
        }
        frame
    }
}

// -------------------------------------------------------------------------------------------------
