use anyhow::Result;
use crossterm::event::{self, Event, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// AppEvent — everything the main loop cares about
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum AppEvent {
    /// Periodic tick for refreshing data.
    Tick,
    /// A keyboard event from the terminal.
    Key(KeyEvent),
    /// The terminal was resized.
    Resize(u16, u16),
}

// ---------------------------------------------------------------------------
// EventHandler — background task that polls crossterm and sends AppEvents
// ---------------------------------------------------------------------------

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<AppEvent>,
    /// Keep a handle so the background task is cancelled when EventHandler is
    /// dropped.
    _task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    /// Spawn the event-polling task.
    ///
    /// `tick_rate_ms` controls how often a `Tick` event is emitted when no
    /// terminal event is available (used to drive periodic data refresh).
    pub fn new(tick_rate_ms: u64) -> Self {
        let tick_rate = Duration::from_millis(tick_rate_ms);
        let (tx, rx) = mpsc::unbounded_channel();

        let task = tokio::spawn(async move {
            loop {
                // `event::poll` is blocking — run it on the threadpool via
                // `spawn_blocking` so we don't stall the tokio runtime.
                let has_event = {
                    let tick = tick_rate;
                    tokio::task::spawn_blocking(move || event::poll(tick).unwrap_or(false))
                        .await
                        .unwrap_or(false)
                };

                if has_event {
                    // Read the event (non-blocking after a successful poll).
                    let read_result = tokio::task::spawn_blocking(event::read).await;

                    match read_result {
                        Ok(Ok(Event::Key(key))) => {
                            if tx.send(AppEvent::Key(key)).is_err() {
                                break; // receiver dropped
                            }
                        }
                        Ok(Ok(Event::Resize(w, h))) => {
                            if tx.send(AppEvent::Resize(w, h)).is_err() {
                                break;
                            }
                        }
                        Ok(Ok(_)) => {
                            // Mouse, paste, focus — ignored for now.
                        }
                        Ok(Err(_)) => {
                            // crossterm read error — skip.
                        }
                        Err(_) => {
                            // join error — skip.
                        }
                    }
                } else {
                    // No terminal event within the tick window — emit Tick.
                    if tx.send(AppEvent::Tick).is_err() {
                        break;
                    }
                }
            }
        });

        Self { rx, _task: task }
    }

    /// Wait for the next event. Returns `Err` only if the channel is closed.
    pub async fn next(&mut self) -> Result<AppEvent> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("event channel closed"))
    }
}
