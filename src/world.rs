//! The seam between the exchange and the world it prices (GDD 5.1, 11).
//!
//! Everything above this module is mode-agnostic: it asks a [`WorldSource`] for
//! the current [`WorldView`] and for the typed changes since the last one, and
//! cannot tell where either came from. Today there is one implementation —
//! [`live::LiveWorld`], reading the shared Mytherra world over HTTP — and a
//! second (an embedded `mytherra-core` ticked in-process) can be added behind
//! the same trait without touching a line downstream.
//!
//! The seam is deliberately **poll-shaped rather than blocking**. A networked
//! source cannot answer "give me the world" synchronously without stalling the
//! frame, so the trait is built around a per-frame [`WorldSource::poll`] that
//! usually returns nothing. An in-process source satisfies that trivially by
//! always being ready; the reverse — retrofitting a blocking trait to a network
//! — is what forks a codebase, which is precisely what this file exists to
//! prevent.
//!
//! This module holds **no write path into Mytherra and never will** (GDD 2, 12).
//! The exchange is strictly downstream of the world.

pub mod delta;
pub mod live;

pub use delta::WorldDelta;

use mytherra_protocol::WorldView;

/// What a world source is currently doing, for the UI's connection banner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceStatus {
    /// No world in hand yet — the first read is still in flight.
    Connecting,
    /// A world is in hand and the last read succeeded.
    Live,
    /// The last read failed. Any previously fetched world is still displayed;
    /// the exchange keeps trading on the last prices it knows, which is what a
    /// real one does when the wire goes quiet.
    Failed(String),
}

impl SourceStatus {
    /// A short line for the status bar.
    pub fn label(&self) -> String {
        match self {
            SourceStatus::Connecting => "opening the wire…".to_owned(),
            SourceStatus::Live => "wire open".to_owned(),
            SourceStatus::Failed(err) => format!("wire down: {err}"),
        }
    }
}

/// A source of world state — the one seam every mode implements (GDD 11).
///
/// The contract is two questions: *what does the world look like now*, and
/// *what changed since you last told me*. Price formation consumes only the
/// second (GDD 5.5); research screens read the first.
pub trait WorldSource {
    /// Advance whatever is in flight by one frame. Returns `Some` exactly when
    /// the world has turned — that is, on the poll where a view arrives with a
    /// higher `tick_count` than the previous one — carrying the typed changes
    /// and the chronicle lines that accompany them.
    ///
    /// Returning `None` is the common case and means "nothing yet"; it is not
    /// an error. Errors surface through [`status`](WorldSource::status) so a
    /// failed poll degrades the banner rather than the trading session.
    fn poll(&mut self, dt: f32) -> Option<WorldDelta>;

    /// The most recent world view, once one has ever arrived.
    fn view(&self) -> Option<&WorldView>;

    /// How far through the world's chronicle this source has read. Persisted
    /// across sessions so a reopened exchange resumes where it stopped instead
    /// of re-reading a century of prose.
    fn cursor(&self) -> u64;

    /// Restore a persisted cursor after a load, so the reopened exchange picks
    /// the chronicle up where it left off.
    fn resume_from(&mut self, cursor: u64);

    /// The health of the link to the world.
    fn status(&self) -> &SourceStatus;
}
