# Amplitude Rust client

An independent Rust port of the supplied Java `Amplitude` client's orchestration.
It has no Statsig dependency. `statsig-amplitude` supplies the optional integration.
Both crates are unpublished while this implementation is under review.

```rust,no_run
use amplitude_rust::{Amplitude, Event, Options};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let amplitude = Amplitude::new(std::env::var("AMPLITUDE_API_KEY")?, Options::default())?;
let mut event = Event::new("purchase");
event.user_id = Some("user-123".into());
amplitude.log_event(event).await?;
amplitude.flush_events().await?;
amplitude.shutdown().await?;
# Ok(())
# }
```

Run inside a Tokio runtime. A single worker serializes uploads and owns the event
buffer; a bounded command channel applies backpressure. Events flush at the upload
threshold or after the configured period from the first buffered event. Dropping
all handles closes the channel and requests a final flush, but explicitly await
`shutdown` before stopping the runtime to guarantee completion.

`get_instance(name, api_key, options)` shares a named client; `""` selects the
default name. The first call sets its configuration. The registry holds weak
references and removes shut-down clients.

Use `configure(|options| { ... }).await` for runtime changes to the server URL,
batch mode, request options, proxy, threshold, upload period, plan, ingestion
metadata, callbacks, middleware, and timeouts. Updates validate atomically.
Per-event plan and ingestion metadata override defaults. Middleware receives a
mutable event and arbitrary JSON extra data and may discard an event. Event and
client callbacks run after the final delivery attempt; discarded events do not
trigger callbacks. Callbacks and middleware must be short and must not block
waiting for another command on the same client.

The HTTP client sends the [Amplitude HTTP V2](https://www.amplitude.com/docs/apis/analytics/http-v2)
or [Batch API](https://www.amplitude.com/docs/apis/analytics/batch-event-upload)
request envelope. It retries network failures, HTTP 408/429, and 5xx responses
with bounded exponential backoff, respects numeric Retry-After headers, and
splits batches on HTTP 413. Insert IDs remain unchanged across retries. Other
HTTP errors are terminal. Limits default to 1,000 events per request; 413 handling
also accommodates the endpoint's byte limit. Set a custom server URL for EU
projects or a reverse proxy. A custom URL takes precedence over batch mode.

`log_event` acknowledges acceptance, not delivery. Automatic flush failures are
reported through callbacks and the Rust `log` facade, and those events are removed
after the final attempt. `send_events` waits for delivery and returns an error so
an external queue can own retries. Retrying after partial success can resend
successful events; retain insert IDs for server deduplication. Flush timeout
applies to each request batch, including its retries, rather than the entire queue.

The Java helpers were not included in the source, so this implements HTTP and
retry behavior explicitly rather than claiming their undocumented behavior.
It does not persist events or interpret partial-ingestion error bodies.
`should_wait(&event)` reports accepted buffer pressure, more than 16,000 events
in a retrying request, and IDs in an active HTTP 429 retry when
`record_throttled_ids` is enabled. Since partial-ingestion error bodies are not
interpreted, every ID in a throttled batch is conservatively marked until that
batch completes. The bounded channel provides additional backpressure. Java executor injection is
replaced by the caller's Tokio runtime; custom logging uses the `log` facade.
API keys are required at construction rather than a separate `init` call.

Run `cargo test -p amplitude-rust` and `cargo clippy -p amplitude-rust --all-targets`.
