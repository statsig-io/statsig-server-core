# Amplitude Rust

Track events and update user properties using the [Amplitude HTTP V2 API](https://amplitude.com/docs/apis/analytics/http-v2).
The [server client contract](spec/amplitude-server-client.wit) defines the client
surface. `track` and `identify` are implemented. The other client methods return
`AmplitudeError::NotImplemented`.

```rust,no_run
use amplitude_rust::{Amplitude, BaseEvent, EventOptions, IdentifyOperation};
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
    amplitude.track(BaseEvent {
        options,
        event_properties: json!({"total": 29.99}).as_object().cloned(),
        ..BaseEvent::new("Purchase Completed")
    }).await?;
    Ok(())
}
```

Await `track` and `identify` on a Tokio runtime to get the ingestion response.
Supply a user ID, a device ID, or both. Use one operation per property and
send `ClearAll` on its own. The client generates a missing timestamp and
insert ID. To retry an operation, pass the same explicit `insert_id` each time.

Pass an optional `Config` to `Amplitude::new`. Both methods use the configured
endpoint, timeout, minimum ID length, opt-out, plan, and ingestion metadata.
The default endpoint is the US HTTP V2 endpoint. Use `ServerZone::Eu` for an
EU project or set `server_url` to pick a specific endpoint.
