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

Await `track` and `identify` on a Tokio runtime to get the HTTP status or an
`AmplitudeError`. A status of zero means tracking was skipped because of
opt-out. Use `AmplitudeError::status_code()` to read the status on failure.
Supply a user ID, a device ID, or both. Use one operation per property and
send `ClearAll` on its own. The client generates a missing timestamp and
insert ID. To retry an operation, pass the same explicit `insert_id` each time.

Pass an optional `Config` to `Amplitude::new`. Set `flush_interval_millis`
to hold concurrent calls for a batch. A full queue flushes sooner.
The defaults are immediate delivery, a queue size of 200, and zero retries.
`flush_max_retries` applies to connection failures, timeouts, and HTTP
408, 429, 500, 502, 503, and 504. Set `log_level` to emit SDK logs to
standard error and `instance_name` to label them.
The default endpoint is the US HTTP V2 endpoint. Use `ServerZone::Eu` for an
EU project or set `server_url` to pick a specific endpoint.
