// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/

//! Implementation of `dharma::Module` for Exhibitor.

// -------------------------------------------------------------------------------------------------

use dharma::{InitResult, Module, ModuleConstructor};
use qualia::{Context, perceptron, Perceptron};
use exhibitor::Exhibitor;

// -------------------------------------------------------------------------------------------------

/// Implementation of `dharma::Module` for Device Manager.
pub struct ExhibitorModule {
    exhibitor: Option<Exhibitor>,
}

// -------------------------------------------------------------------------------------------------

impl ExhibitorModule {
    /// `ExhibitorModule` constructor.
    pub fn new() -> Self {
        ExhibitorModule { exhibitor: None }
    }
}

// -------------------------------------------------------------------------------------------------

impl Module for ExhibitorModule {
    type T = Perceptron;
    type C = Context;

    fn initialize(&mut self, context: &mut Self::C) -> InitResult {
        log_info1!("Starting Exhibitor module");
        self.exhibitor = Some(Exhibitor::new(context.get_signaler().clone(),
                                             context.get_coordinator().clone()));
        vec![perceptron::NOTIFY,
             perceptron::PAGE_FLIP,
             perceptron::OUTPUT_FOUND,
             perceptron::COMMAND,
             perceptron::INPUT_POINTER_MOTION,
             perceptron::INPUT_POINTER_POSITION,
             perceptron::INPUT_POINTER_BUTTON,
             perceptron::INPUT_POINTER_POSITION_RESET,
             perceptron::CURSOR_SURFACE_CHANGE,
             perceptron::SURFACE_READY,
             perceptron::SURFACE_DESTROYED,
             perceptron::KEYBOARD_FOCUS_CHANGED]
    }

    fn execute(&mut self, package: &Self::T) {
        if let Some(ref mut exhibitor) = self.exhibitor {
            match *package {
                Perceptron::Notify => exhibitor.on_notify(),
                Perceptron::OutputFound(bundle) => exhibitor.on_output_found(bundle),
                Perceptron::PageFlip(id) => exhibitor.on_pageflip(id),
                Perceptron::Command(ref command) => exhibitor.on_command(command.clone()),

                Perceptron::InputPointerMotion(ref vector) => exhibitor.on_motion(vector.clone()),
                Perceptron::InputPointerPosition(ref pos) => exhibitor.on_position(pos.clone()),
                Perceptron::InputPointerButton(ref btn) => exhibitor.on_button(btn.clone()),
                Perceptron::InputPointerPositionReset => exhibitor.on_position_reset(),

                Perceptron::CursorSurfaceChange(sid) => exhibitor.on_cursor_surface_change(sid),

                Perceptron::SurfaceReady(sid) => exhibitor.on_surface_ready(sid),
                Perceptron::SurfaceDestroyed(sid) => exhibitor.on_surface_destroyed(sid),

                Perceptron::KeyboardFocusChanged(_, sid) => {
                    exhibitor.on_keyboard_focus_changed(sid)
                }
                _ => {}
            }
        }
    }

    fn finalize(&mut self) {
        log_info1!("Finalized Exhibitor module");
    }
}

// -------------------------------------------------------------------------------------------------

pub struct ExhibitorModuleConstructor {}

// -------------------------------------------------------------------------------------------------

impl ExhibitorModuleConstructor {
    /// Constructs new `ExhibitorModuleConstructor`.
    pub fn new() -> Box<ModuleConstructor<T = Perceptron, C = Context>> {
        Box::new(ExhibitorModuleConstructor {})
    }
}

// -------------------------------------------------------------------------------------------------

impl ModuleConstructor for ExhibitorModuleConstructor {
    type T = Perceptron;
    type C = Context;

    fn construct(&self) -> Box<Module<T = Self::T, C = Self::C>> {
        Box::new(ExhibitorModule::new())
    }
}

// -------------------------------------------------------------------------------------------------
