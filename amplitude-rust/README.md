# Amplitude Rust

Track server-side events with the [Amplitude HTTP V2 API](https://amplitude.com/docs/apis/analytics/http-v2).

```rust,no_run
use amplitude_rust::{Amplitude, AmplitudeEvent};
use serde_json::json;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("AMPLITUDE_API_KEY")?;
    let amplitude = Amplitude::new(&api_key, None)?;

    let mut event = AmplitudeEvent::new("Purchase Completed");
    event.user_id = Some("user-123".into());
    event.event_properties = json!({"total": 29.99, "currency": "USD"})
        .as_object()
        .cloned();

    amplitude.track(event).await?;
    Ok(())
}
```

Reuse the client across calls and await `track` on a Tokio runtime. Each call
sends one event immediately and returns after the API responds.

Set a user ID, a device ID, or both. Amplitude's default minimum ID length is
five characters. The client fills in a missing event timestamp and insert ID
when sending. Set `insert_id` explicitly and reuse it if you retry an event.

Pass `Some(AmplitudeOptions { .. })` to `Amplitude::new` to override the endpoint
or the default 10-second timeout. For EU projects, use
`https://api.eu.amplitude.com/2/httpapi` as the `server_url`.
