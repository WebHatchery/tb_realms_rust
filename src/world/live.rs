//! The Live world source: the shared Mytherra world, read over HTTP (GDD 5.1).
//!
//! This trades the same world every deity is nudging. It reads it through
//! `mytherra-server`'s **spectator** endpoints — `GET /spectate` and
//! `GET /spectate/events?since=` — which serve a projection with every
//! visibility scope revealed and no action verb or betting market at all. The
//! exchange therefore sees the whole world and, by construction of the endpoint
//! it is reading, can change none of it (GDD Pillar 2, §12). There is no
//! `POST /action` in this file and there never will be.
//!
//! The transport uses the toolkit's shared `net` feature: `quad-net` gives one
//! poll-based, non-blocking API across native (a background thread) and WASM
//! (macroquad's own `sapp-jsutils` interop, *not* wasm-bindgen, so it coexists
//! with the WebGL build). Every request is a [`Pending<T>`] polled once per
//! frame; nothing ever blocks the game loop.
//!
//! WASM runtime caveat, inherited from the shared transport: the browser shim
//! only resolves an HTTP 200, so a refused connection never resolves on its own
//! — which is what [`Pending::poll_timed`] exists to catch.

use crate::world::delta::{diff, WorldDelta};
use crate::world::{SourceStatus, WorldSource};
use macroquad_toolkit::net::{HttpClient, Pending};
use mytherra_protocol::{EventsDelta, WorldView};

/// The header carrying the read-only spectator secret, matched by the server's
/// `SPECTATOR_TOKEN_HEADER`.
const SPECTATOR_TOKEN_HEADER: &str = "X-Spectator-Token";

/// A handle to Mytherra's read-only spectator endpoints.
pub struct SpectatorClient {
    api: HttpClient,
}

impl SpectatorClient {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            api: HttpClient::new(base_url).with_header(SPECTATOR_TOKEN_HEADER, token),
        }
    }

    /// `GET /spectate` — the whole world, read-only.
    pub fn fetch_world(&self) -> Pending<WorldView> {
        self.api.get("/spectate")
    }

    /// `GET /spectate/events?since=` — the chronicle since `cursor`.
    pub fn fetch_events(&self, since: u64) -> Pending<EventsDelta> {
        self.api.get(&format!("/spectate/events?since={since}"))
    }
}

/// What the live source is waiting on. The two reads are sequenced rather than
/// issued together on purpose: the chronicle is only fetched once a *newer*
/// world has actually arrived, so the prose a settlement reports always
/// belongs to the turning it accompanies instead of to whatever was in flight
/// nearby.
enum Stage {
    /// Between polls, counting down to the next read.
    Waiting(f32),
    /// A world read is in flight.
    Fetching(Box<Pending<WorldView>>),
    /// The world turned; fetching the chronicle that goes with it. The already
    /// diffed delta waits here for its prose.
    FetchingEvents {
        request: Pending<EventsDelta>,
        delta: Box<WorldDelta>,
    },
}

/// The shared Mytherra world, polled over HTTP.
pub struct LiveWorld {
    client: SpectatorClient,
    stage: Stage,
    /// The newest world in hand. `None` until the first read lands.
    view: Option<WorldView>,
    /// The chronicle cursor last acknowledged.
    cursor: u64,
    status: SourceStatus,
    poll_seconds: f32,
    timeout_seconds: f32,
}

impl LiveWorld {
    /// Open the wire. The first read is issued on the first [`poll`], so
    /// construction never blocks and never fails.
    pub fn connect(client: SpectatorClient, poll_seconds: f32, timeout_seconds: f32) -> Self {
        Self {
            client,
            // Zero so the opening read goes out on the very first frame rather
            // than after an idle interval.
            stage: Stage::Waiting(0.0),
            view: None,
            cursor: 0,
            status: SourceStatus::Connecting,
            poll_seconds: poll_seconds.max(0.1),
            timeout_seconds: timeout_seconds.max(1.0),
        }
    }

    /// Note a failed read without discarding the world already in hand — a
    /// broker keeps trading on the last prices it knows when the wire goes
    /// quiet.
    fn fail(&mut self, err: String) -> Option<WorldDelta> {
        self.status = SourceStatus::Failed(err);
        self.stage = Stage::Waiting(self.poll_seconds);
        None
    }

    /// Take delivery of a world read. Returns a delta only when the world has
    /// genuinely turned; a re-read of the same tick is not a settlement.
    fn accept(&mut self, fresh: WorldView) -> Option<WorldDelta> {
        self.status = SourceStatus::Live;

        let turned = match &self.view {
            // The first world to arrive is a baseline, not a turning: there is
            // no previous view to diff it against, and treating it as a delta
            // would price the entire world as if it had just moved.
            None => None,
            Some(previous) if fresh.tick_count > previous.tick_count => {
                Some(diff(previous, &fresh))
            }
            Some(_) => None,
        };

        self.view = Some(fresh);

        match turned {
            Some(delta) => {
                self.stage = Stage::FetchingEvents {
                    request: self.client.fetch_events(self.cursor),
                    delta: Box::new(delta),
                };
                None
            }
            None => {
                self.stage = Stage::Waiting(self.poll_seconds);
                None
            }
        }
    }
}

impl WorldSource for LiveWorld {
    fn poll(&mut self, dt: f32) -> Option<WorldDelta> {
        match &mut self.stage {
            Stage::Waiting(remaining) => {
                *remaining -= dt;
                if *remaining <= 0.0 {
                    self.stage = Stage::Fetching(Box::new(self.client.fetch_world()));
                }
                None
            }
            Stage::Fetching(request) => match request.poll_timed(dt, self.timeout_seconds) {
                Some(Ok(view)) => self.accept(view),
                Some(Err(err)) => self.fail(err),
                None => None,
            },
            Stage::FetchingEvents { request, delta } => {
                match request.poll_timed(dt, self.timeout_seconds) {
                    Some(Ok(events)) => {
                        let mut delta = std::mem::take(&mut **delta);
                        delta.events = events.events;
                        self.cursor = events.cursor;
                        self.stage = Stage::Waiting(self.poll_seconds);
                        Some(delta)
                    }
                    // The world still turned even if its chronicle didn't
                    // arrive. Report the delta with no prose rather than
                    // dropping a settlement — the numbers are the truth and
                    // the chronicle is the newspaper (GDD 5.5).
                    Some(Err(err)) => {
                        let delta = std::mem::take(&mut **delta);
                        self.status = SourceStatus::Failed(format!("chronicle unread: {err}"));
                        self.stage = Stage::Waiting(self.poll_seconds);
                        Some(delta)
                    }
                    None => None,
                }
            }
        }
    }

    fn view(&self) -> Option<&WorldView> {
        self.view.as_ref()
    }

    fn cursor(&self) -> u64 {
        self.cursor
    }

    fn resume_from(&mut self, cursor: u64) {
        self.cursor = cursor;
    }

    fn status(&self) -> &SourceStatus {
        &self.status
    }
}

#[cfg(test)]
mod tests;
