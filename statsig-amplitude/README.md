# Statsig Amplitude integration

An opt-in Rust library that forwards Statsig custom events and exposure events to
the independent `amplitude-rust` client. Nothing changes for existing Statsig SDK
users, and neither crate is currently published.

```rust,no_run
use amplitude_rust::{Amplitude, Options};
use serde_json::json;
use statsig_amplitude::StatsigAmplitudeAdapter;
use statsig_rust::{Statsig, StatsigHttpEventLoggingAdapter, StatsigOptions};
use std::sync::Arc;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let sdk_key = std::env::var("STATSIG_SERVER_SECRET")?;
let amplitude = Amplitude::new(std::env::var("AMPLITUDE_API_KEY")?, Options {
    // Statsig IDs may be shorter than Amplitude's default five characters.
    request_options: Some(json!({"min_id_length": 1})),
    ..Options::default()
})?;
let primary = Arc::new(StatsigHttpEventLoggingAdapter::new(&sdk_key, None));
let adapter = Arc::new(StatsigAmplitudeAdapter::new(primary, amplitude));
let options = StatsigOptions {
    event_logging_adapter: Some(adapter),
    ..StatsigOptions::default()
};
let statsig = Statsig::new(&sdk_key, Some(Arc::new(options)));
// Initialize and use Statsig normally, then await Statsig shutdown.
# Ok(())
# }
```

Wrap the same primary adapter you would otherwise use, including its existing
network configuration. The adapter preserves its lifecycle and background flush
preference. Statsig owns batching; forwarding calls `send_events` directly so it
waits for Amplitude delivery without adding a second delayed buffer.

| Statsig | Amplitude |
| --- | --- |
| `eventName` | `event_type` |
| `user.userID` | `user_id` |
| `time` | `time` in milliseconds |
| `value`, `metadata`, `secondaryExposures`, `statsigMetadata` | Namespaced under `event_properties.statsig` |
| Mapped event content | Deterministic SHA-256 `insert_id` |

Events without a nonempty string `userID` are skipped, including anonymous
internal diagnostics. User attributes, custom user attributes, emails, IPs,
private attributes, and request-level SDK metadata are not forwarded. Explicit
event metadata is forwarded, so callers control its contents. IDs remain stable
across Statsig retry counters and JSON object ordering. Identical mapped events
for the same user at the same millisecond share an ID and will be deduplicated.

The primary adapter must succeed before forwarding starts. Amplitude failures
are best effort: callbacks and logging report them, while Statsig retains its
successful result. This avoids resending successful Statsig uploads because a
secondary destination failed. Forwarding adds network latency to a Statsig flush,
bounded per Amplitude batch by its configured timeout. There is no durable
outbox or independent retry queue after Amplitude's configured attempts finish.
The adapter owns shutdown of the supplied Amplitude client; do not share that
client with another adapter that has a different lifecycle.

Run `cargo test -p statsig-amplitude` and
`cargo clippy -p statsig-amplitude --all-targets`.
