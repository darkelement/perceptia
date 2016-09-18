// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/

//! Compositor is manager of surfaces. Cares about placing and manipulating them according to
//! user-defined strategies.

// -------------------------------------------------------------------------------------------------

use qualia::{Coordinator, SurfaceId, SurfaceInfo};

use surface_history::SurfaceHistory;
use frames::{self, Frame};
use frames::searching::Searching;
use frames::settling::Settling;

// -------------------------------------------------------------------------------------------------

macro_rules! try_get_surface {
    ($compositor:expr, $sid:ident) => {
        match $compositor.coordinator.get_surface($sid) {
            Some(surface) => surface,
            None => {
                log_warn2!("Surface {} not found!", $sid);
                return
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------

/// Structure describing strategic decision about how to handle new surface.
struct ManageDecision {
    /// Target frame where new surface should be settled.
    target: Frame,

    /// Geometry of new frame.
    geometry: frames::Geometry,

    /// True if new frame should be selected. False otherwise.
    selection: bool,
}

// -------------------------------------------------------------------------------------------------

/// Compositor main structure.
pub struct Compositor {
    history: SurfaceHistory,
    coordinator: Coordinator,
    root: Frame,
    selection: Option<Frame>,
}

// -------------------------------------------------------------------------------------------------

impl Compositor {
    /// `Compositor` constructor.
    pub fn new(coordinator: Coordinator) -> Self {
        Compositor {
            history: SurfaceHistory::new(),
            coordinator: coordinator,
            root: Frame::new_root(),
            selection: None,
        }
    }

    /// Handles new surface by settling it in frame tree, adding to history and notifying
    /// coordinator.
    pub fn manage_surface(&mut self, sid: SurfaceId) {
        // Get surface
        let surface = try_get_surface!(self, sid);

        // Consult about placement strategy
        let decision = self.choose_target(&surface);

        // Settle and optionally select new frame
        let frame = Frame::new_leaf(sid, decision.geometry);
        frame.settle(&decision.target, &self.coordinator);
        if decision.selection {
            self.select(Some(frame));
        }

        // Finalize
        self.history.add(sid);
        self.coordinator.notify();
    }
}

// -------------------------------------------------------------------------------------------------

// Private methods
impl Compositor {
    /// Set given frame as selected.
    fn select(&mut self, frame: Option<Frame>) {
        self.selection = frame
    }

    /// Get selected frame.
    fn get_selection(&self) -> Frame {
        self.selection.clone().unwrap()
    }

    /// Decide how to handle new surface.
    fn choose_target(&self, surface: &SurfaceInfo) -> ManageDecision {
        if surface.parent_sid.is_valid() {
            // FIXME: Choosing surface target should be configurable.
            ManageDecision {
                target: self.get_selection().find_buildable().unwrap(),
                geometry: frames::Geometry::Stacked,
                selection: true,
            }
        } else {
            ManageDecision {
                target: self.get_selection().find_top().unwrap(),
                geometry: frames::Geometry::Vertical,
                selection: true,
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------