# Amplitude Analytics (initial implementation)

```rust,no_run
use statsig_rust::{Amplitude, AmplitudeEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
let mut amplitude = Amplitude::new();
amplitude.init("AMPLITUDE_API_KEY", None)?;
amplitude.track(AmplitudeEvent {
    user_id: Some("user-123".into()),
    event_properties: Some(serde_json::json!({"title": "Hello"}).as_object().unwrap().clone()),
    ..AmplitudeEvent::new("Song Played")
}).await?;
Ok(())
}
```

`init` configures the client locally. `track` sends one event to Amplitude's HTTP V2 API and waits for acceptance. Events require `event_type` and either `user_id` or `device_id`; Amplitude's default minimum ID length is five characters. Missing timestamps and insert IDs are generated automatically. Event and user properties, session IDs, and explicit timestamps and insert IDs are supported.

`AmplitudeOptions` supports `server_url` (default `https://api2.amplitude.com/2/httpapi`) and `timeout_ms` (default 10,000). Tracking failures return `AmplitudeError`. There is no buffering, automatic retry, or background work in this iteration; callers should await each track and handle errors. Reuse an explicit insert ID when retrying an event.

The wire format follows [Amplitude-Java](https://github.com/amplitude/Amplitude-Java/tree/main), particularly `Amplitude.init`, `Event`, and `HttpCall`. Its batching, middleware, identify, and revenue APIs are outside this iteration.
