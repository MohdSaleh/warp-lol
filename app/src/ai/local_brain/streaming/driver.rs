use futures_util::StreamExt;
use http_client::EventSourceStream;
use tokio::sync::mpsc;
use tokio::time::timeout;

use super::config::StreamTimeoutConfig;
use super::error::LocalStreamError;
use super::protocol::{CanonicalStreamEvent, RawSseEvent, StreamProtocolDecoder};

pub enum StreamExit {
    Cancelled,
    Complete,
    Failed(LocalStreamError),
}

pub struct LocalStreamDriver {
    event_source: EventSourceStream,
    decoder: Box<dyn StreamProtocolDecoder + Send>,
    config: StreamTimeoutConfig,
}

impl LocalStreamDriver {
    pub fn new(
        event_source: EventSourceStream,
        decoder: Box<dyn StreamProtocolDecoder + Send>,
        config: StreamTimeoutConfig,
    ) -> Self {
        Self {
            event_source,
            decoder,
            config,
        }
    }

    pub async fn run(
        mut self,
        tx: mpsc::Sender<CanonicalStreamEvent>,
        cancellation: tokio_util::sync::CancellationToken,
    ) -> StreamExit {
        let connect_timeout = self.config.connect_timeout;
        let first_event_timeout = self.config.first_event_timeout;
        let idle_timeout = self.config.idle_timeout;
        let total_timeout = self.config.total_timeout;

        let start_time = instant::Instant::now();
        let mut first_event_received = false;

        loop {
            if let Some(total_to) = total_timeout {
                if start_time.elapsed() >= total_to {
                    return StreamExit::Failed(LocalStreamError::TotalTimeout);
                }
            }

            let timeout_duration = if !first_event_received {
                first_event_timeout
            } else {
                idle_timeout
            };

            let next_event_fut = timeout(timeout_duration, self.event_source.next());

            tokio::select! {
                biased;

                _ = cancellation.cancelled() => {
                    return StreamExit::Cancelled;
                }

                next_res = next_event_fut => {
                    match next_res {
                        Ok(Some(Ok(reqwest_eventsource::Event::Open))) => {
                            if !first_event_received {
                                first_event_received = true;
                            }
                            match self.decoder.decode(RawSseEvent::Open) {
                                Ok(canonical_events) => {
                                    for ev in canonical_events {
                                        if tx.send(ev).await.is_err() {

                                            return StreamExit::Complete;
                                        }
                                    }
                                }
                                Err(err) => {

                                    return StreamExit::Failed(err);
                                }
                            }
                        }
                        Ok(Some(Ok(reqwest_eventsource::Event::Message(msg)))) => {
                            if !first_event_received {
                                first_event_received = true;
                            }
                            match self.decoder.decode(RawSseEvent::Message(msg.data)) {
                                Ok(canonical_events) => {
                                    for ev in canonical_events {
                                        let is_done = matches!(ev, CanonicalStreamEvent::DoneMarker);
                                        if tx.send(ev).await.is_err() {
                                            return StreamExit::Complete;
                                        }
                                        if is_done {
                                            return StreamExit::Complete;
                                        }
                                    }
                                }
                                Err(err) => {

                                    return StreamExit::Failed(err);
                                }
                            }
                        }
                        Ok(Some(Err(reqwest_eventsource::Error::StreamEnded))) => {
                            return StreamExit::Complete;
                        }
                        Ok(Some(Err(e))) => {
                            let stream_err = match e {
                                reqwest_eventsource::Error::InvalidStatusCode(status, response) => {
                                    let code = status.as_u16();
                                    if code == 429 {
                                        let retry_after_secs = response
                                            .headers()
                                            .get("retry-after")
                                            .and_then(|h| h.to_str().ok())
                                            .and_then(|s| s.parse::<u64>().ok());
                                        LocalStreamError::RateLimitExceeded {
                                            status: code,
                                            retry_after_secs,
                                        }
                                    } else if code == 400 || code == 413 {
                                        let body = response.text().await.unwrap_or_default();
                                        let body_lower = body.to_lowercase();
                                        if body_lower.contains("context_length_exceeded")
                                            || body_lower.contains("context length")
                                            || body_lower.contains("maximum context")
                                            || body_lower.contains("too many tokens")
                                            || body_lower.contains("prompt is too long")
                                            || body_lower.contains("string_above_max_length")
                                        {
                                            LocalStreamError::ContextLimitExceeded(body)
                                        } else {
                                            LocalStreamError::InvalidStatusCode(code)
                                        }
                                    } else {
                                        LocalStreamError::InvalidStatusCode(code)
                                    }
                                }
                                reqwest_eventsource::Error::InvalidContentType(..) => {
                                    LocalStreamError::InvalidContentType
                                }
                                other => LocalStreamError::MalformedFrame(other.to_string()),
                            };
                            return StreamExit::Failed(stream_err);
                        }
                        Ok(None) => {
                            return StreamExit::Complete;
                        }
                        Err(_) => {
                            if !first_event_received {
                                return StreamExit::Failed(LocalStreamError::FirstEventTimeout);
                            } else {
                                return StreamExit::Failed(LocalStreamError::IdleTimeout);
                            }
                        }
                    }
                }
            }
        }
    }
}
