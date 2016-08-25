// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0. If a copy of
// the MPL was not distributed with this file, You can obtain one at http://mozilla.org/MPL/2.0/

//! `Signaler` allows to send events (notifications) to other parts of application.
//!
//! Together with `Samsara` constitutes higher level framework over `Sender` and `Receiver`.

// -------------------------------------------------------------------------------------------------

use std::clone::Clone;
use std::collections::btree_map::BTreeMap as Map;
use std::sync::{Arc, Mutex};

use bridge::{self, Receiver};

// -------------------------------------------------------------------------------------------------

pub type SignalId = usize;

// -------------------------------------------------------------------------------------------------

/// Enum used for communication between `Signaler` and event loop.
#[derive(Clone, PartialEq, Debug)]
pub enum Event<P: bridge::Transportable> {
    Package { id: SignalId, package: P },
    Terminate,
}

// -------------------------------------------------------------------------------------------------

impl<P: bridge::Transportable> bridge::Transportable for Event<P> {}

// -------------------------------------------------------------------------------------------------

/// Helper structure constituting shared memory between `Signaler`s from different threads.
struct InnerSignaler<P: bridge::Transportable> {
    map: Map<SignalId, bridge::Sender<Event<P>>>,
    registry: Vec<bridge::Sender<Event<P>>>,
}

// -------------------------------------------------------------------------------------------------

/// `Signaler` allows to send events (or notifications) to other parts of application possibly
/// running in different threads. `Signaler` is generic over sent data type, meaning that it can
/// convey only data of given type (therefore easiest is to choose enum).
pub struct Signaler<P: bridge::Transportable> {
    inner: Arc<Mutex<InnerSignaler<P>>>,
}

// -------------------------------------------------------------------------------------------------

impl<P: bridge::Transportable> Signaler<P> {
    /// `Signaler` constructor.
    pub fn new() -> Self {
        Signaler {
            inner: Arc::new(Mutex::new(InnerSignaler {
                map: Map::new(),
                registry: Vec::new(),
            })),
        }
    }

    /// Subscribe given `receiver` for a signal `id`.
    pub fn subscribe(&mut self, id: SignalId, receiver: &Receiver<Event<P>>) {
        let mut mine = self.inner.lock().unwrap();

        if mine.map.contains_key(&id) {
            // If someone is already connected, connect next receiver
            match mine.map.get_mut(&id) {
                Some(ref mut sender) => {
                    bridge::connect(sender, receiver);
                }
                None => {} // FIXME warn
            }
        } else {
            // If this is first subscription, create sender and connect receiver
            let mut sender = bridge::Sender::new();
            bridge::connect(&mut sender, receiver);
            mine.map.insert(id, sender);
        }
    }

    /// Register `receiver` for control instructions like request to terminate.
    pub fn register(&mut self, receiver: &bridge::Receiver<Event<P>>) {
        let mut mine = self.inner.lock().unwrap();
        let mut sender = bridge::Sender::new();
        bridge::connect(&mut sender, receiver);
        mine.registry.push(sender);
    }

    /// Emit signal `id` containing data `package`. All subscribed `Receiver`s will be notified.
    pub fn emit(&mut self, id: SignalId, package: P) {
        let mut mine = self.inner.lock().unwrap();

        // Find sender assigned to signal
        match mine.map.get_mut(&id) {
            Some(sender) => {
                // Send package to all connected receivers
                sender.send(Event::Package {
                    id: id,
                    package: package,
                });
            }
            None => {
                // No one to notify
            }
        }
    }

    /// Send `Terminate` instruction to registered `Receiver`s indicating `Signaler` (possibly whole
    /// application) is going to shut down.
    pub fn terminate(&mut self) {
        let mut mine = self.inner.lock().unwrap();
        for mut sender in mine.registry.iter_mut() {
            sender.send(Event::Terminate);
        }
    }
}

// -------------------------------------------------------------------------------------------------

impl<P: bridge::Transportable> Clone for Signaler<P> {
    fn clone(&self) -> Self {
        Signaler { inner: self.inner.clone() }
    }
}

// -------------------------------------------------------------------------------------------------
