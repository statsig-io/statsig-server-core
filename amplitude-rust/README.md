# Amplitude Rust

Track events and update user properties using the [Amplitude HTTP V2 API](https://amplitude.com/docs/apis/analytics/http-v2).
The [server client contract](spec/amplitude-server-client.md) describes
`track` and `identify` and links to their TypeSpec data models.

```rust,no_run
use amplitude_rust::{Amplitude, Event, EventOptions, IdentifyOperation};
use serde_json::json;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("AMPLITUDE_API_KEY")?;
    let amplitude = Amplitude::new(&api_key, None)?;
    let operations = vec![
        IdentifyOperation::Set {
            key: "plan".into(),
            value: json!("pro"),
        },
        IdentifyOperation::Add {
            key: "logins".into(),
            value: 1.0,
        },
    ];
    let options = EventOptions {
        user_id: Some("user-123".into()),
        ..EventOptions::default()
    };
    amplitude.identify(operations, options.clone(), None).await?;
    amplitude.track(Event {
        options,
        event_properties: json!({"total": 29.99}).as_object().cloned(),
        ..Event::new("Purchase Completed")
    }).await?;
    Ok(())
}
```

Await `track` and `identify` on a Tokio runtime. With the default flush
interval of zero, the calls return the HTTP status or an `AmplitudeError`.
With a positive `flush_interval_millis`, they return zero once the default
destination accepts the event. The worker sends it when the interval elapses
or the queue fills. Filtered and opted-out events also return zero.

Use `track_with_result` or `identify_with_result` to await an individual
delivery at any interval. Use `AmplitudeError::status_code()` to read the
status on failure. Await `flush()` to send pending batches immediately or
`shutdown()` to stop accepting events and drain them before the Tokio
runtime exits. Both methods return outstanding batch outcomes grouped by
destination.

Supply a user ID, a device ID, or both. Use one operation per property and
send `ClearAll` on its own. The client generates a missing timestamp and
insert ID. To retry an operation, pass the same explicit `insert_id` each time.

Pass an optional `Config` to `Amplitude::new`.
The defaults are immediate delivery, a queue size of 200, and zero retries.
`flush_max_retries` applies to connection failures, timeouts, and HTTP
408, 429, 500, 502, 503, and 504. Set `log_level` to emit SDK logs to
standard error and `instance_name` to label them.
The default endpoint is the US HTTP V2 endpoint. Use `ServerZone::Eu` for an
EU project or set `server_url` to pick a specific endpoint.

The client registers an `AmplitudeDestinationPlugin` by default. Use
`add(Plugin::Before(...))`, `add(Plugin::Enrichment(...))`, or
`add(Plugin::Destination(...))` to register a plugin. Use its returned ID
with `remove`. Before and Enrichment plugins return `None` to filter an
event. The timeline invokes each destination and uses the first one's
outcome. See the [Timeline contract](spec/amplitude-timeline.md) and
[plugin contract](spec/amplitude-plugins.md).

The regular test suite skips the live smoke tests. Run them manually from the
repository root with `AMPLITUDE_API_KEY` in your shell. One case sends an
identify event and three `SDK Smoke Test` events after a three-second interval.
The shutdown case queues an `SDK Smoke Test Shutdown` event with a ten-second
interval and confirms that shutdown sends it before the timer fires. Set
`AMPLITUDE_SERVER_ZONE=eu` for an EU project. Find the test events using the
printed user ID or the `smoke_run_id` event property.

```sh
cargo test --locked -p amplitude-rust --test smoke -- --ignored --nocapture
```
