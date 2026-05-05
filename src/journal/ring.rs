//! Fixed-capacity event journal.
//!
//! The journal stores recent lifecycle events for diagnostics and replay. It
//! does not own subscribers or exporters.

use crate::event::payload::SupervisorEvent;
use crate::event::time::EventSequence;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Fixed-capacity lifecycle event journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventJournal {
    /// Maximum number of events retained in memory.
    pub capacity: usize,
    /// Recent events in oldest-to-newest order.
    pub events: VecDeque<SupervisorEvent>,
    /// Number of events dropped because capacity was full.
    pub dropped_count: u64,
    /// Last sequence written to the journal.
    pub last_sequence: Option<EventSequence>,
}

impl EventJournal {
    /// Creates an event journal with fixed capacity.
    ///
    /// # Arguments
    ///
    /// - `capacity`: Maximum number of events retained. Zero is allowed and
    ///   drops every pushed event.
    ///
    /// # Returns
    ///
    /// Returns an empty [`EventJournal`].
    ///
    /// # Examples
    ///
    /// ```
    /// let journal = rust_supervisor::journal::ring::EventJournal::new(2);
    /// assert_eq!(journal.capacity, 2);
    /// ```
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            events: VecDeque::with_capacity(capacity),
            dropped_count: 0,
            last_sequence: None,
        }
    }

    /// Pushes an event and drops the oldest event when full.
    ///
    /// # Arguments
    ///
    /// - `event`: Event that should be retained when capacity permits.
    ///
    /// # Returns
    ///
    /// Returns the new dropped count.
    pub fn push(&mut self, event: SupervisorEvent) -> u64 {
        self.last_sequence = Some(event.sequence);
        if self.capacity == 0 {
            self.dropped_count = self.dropped_count.saturating_add(1);
            return self.dropped_count;
        }
        if self.events.len() == self.capacity {
            self.events.pop_front();
            self.dropped_count = self.dropped_count.saturating_add(1);
        }
        self.events.push_back(event);
        self.dropped_count
    }

    /// Returns recent events with newest events retained.
    ///
    /// # Arguments
    ///
    /// - `limit`: Maximum number of events to return.
    ///
    /// # Returns
    ///
    /// Returns events in oldest-to-newest order.
    ///
    /// # Examples
    ///
    /// ```
    /// let journal = rust_supervisor::journal::ring::EventJournal::new(4);
    /// assert!(journal.recent(2).is_empty());
    /// ```
    pub fn recent(&self, limit: usize) -> Vec<SupervisorEvent> {
        let skip = self.events.len().saturating_sub(limit);
        self.events.iter().skip(skip).cloned().collect()
    }

    /// Returns the number of retained events.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns the current event count.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Reports whether the journal has no retained events.
    ///
    /// # Arguments
    ///
    /// This function has no arguments.
    ///
    /// # Returns
    ///
    /// Returns `true` when the journal is empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}
