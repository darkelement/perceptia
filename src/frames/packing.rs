// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/

//! This module contains extra settling functionality for `frames::Frame`.

// -------------------------------------------------------------------------------------------------

use qualia::{Position, Size, Vector};
use qualia::{SurfaceAccess, surface_state};

use frame::{Frame, Geometry};

// -------------------------------------------------------------------------------------------------

/// Extension trait for `Frame` adding more packing functionality.
pub trait Packing {
    /// TODO: Implement relaxing. Currently relaxing is equivalent to homogenizing.
    fn relax(&mut self, sa: &mut SurfaceAccess);

    /// Make all subsurfaces have the same size and proper layout.
    /// Homogenizing works only on directed frames.
    fn homogenize(&mut self, sa: &mut SurfaceAccess);

    /// Set size of the frame and resize its subframe accordingly.
    fn set_size(&mut self, size: Size, sa: &mut SurfaceAccess);

    /// Set new position for given frame and move it subframes accordingly.
    fn set_position(&mut self, pos: Position);

    /// Move the frame and all subframes by given vector.
    fn move_with_contents(&mut self, vector: Vector);

    /// Remove given frame and relax old parent.
    fn remove_self(&mut self, sa: &mut SurfaceAccess);
}

// -------------------------------------------------------------------------------------------------

impl Packing for Frame {
    fn relax(&mut self, sa: &mut SurfaceAccess) {
        self.homogenize(sa);
    }

    fn homogenize(&mut self, sa: &mut SurfaceAccess) {
        let len = self.count_children();
        if len < 1 {
            return;
        }

        // Decide how to resize and move twigs
        let mut size = Size::new(0, 0);
        let mut increment = Vector::new(0, 0);
        match self.get_geometry() {
            Geometry::Stacked => {
                size = self.get_size();
            }
            Geometry::Vertical => {
                size.width = self.get_size().width;
                size.height = self.get_size().height / len;
                increment.y = size.height as isize;
            }
            Geometry::Horizontal => {
                size.height = self.get_size().height;
                size.width = self.get_size().width / len;
                increment.x = size.width as isize;
            }
            Geometry::Floating => {
                // Nothing to do for not-directed frames
                return;
            }
        }

        // Resize and reposition all subframes recursively
        let mut pos = self.get_position();
        for mut frame in self.space_iter() {
            frame.set_size(size.clone(), sa);
            frame.set_position(pos.clone());
            pos = pos + increment.clone();
        }
    }

    fn set_size(&mut self, size: Size, sa: &mut SurfaceAccess) {
        // Set size for given frame.
        let old_size = self.get_size();
        self.set_plumbing_size(size.clone());
        sa.reconfigure(self.get_sid(), size.clone(), surface_state::MAXIMIZED);

        // Set size to frames children.
        match self.get_geometry() {
            Geometry::Horizontal => {
                if old_size.width == size.width {
                    for mut frame in self.space_iter() {
                        let mut frame_size = frame.get_size();
                        frame_size.height = size.height;
                        frame.set_size(frame_size, sa);
                    }
                } else {
                    self.relax(sa);
                }
            }
            Geometry::Vertical => {
                if old_size.height == size.height {
                    for mut frame in self.space_iter() {
                        let mut frame_size = frame.get_size();
                        frame_size.width = size.width;
                        frame.set_size(frame_size, sa);
                    }
                } else {
                    self.relax(sa);
                }
            }
            _ => {
                for mut frame in self.space_iter() {
                    frame.set_size(size.clone(), sa);
                }
            }
        }
    }

    fn set_position(&mut self, pos: Position) {
        let vector = pos - self.get_position();
        self.move_with_contents(vector);
    }

    fn move_with_contents(&mut self, vector: Vector) {
        // Update frames position
        let new_position = self.get_position() + vector.clone();
        self.set_plumbing_position(new_position);

        // Move all subframes
        for mut frame in self.space_iter() {
            frame.move_with_contents(vector.clone());
        }
    }

    fn remove_self(&mut self, sa: &mut SurfaceAccess) {
        if let Some(ref mut parent) = self.get_parent() {
            self.remove();
            parent.relax(sa);
        }
    }
}

// -------------------------------------------------------------------------------------------------
