# Amplitude Analytics

The Rust implementation follows [Amplitude-Java](https://github.com/amplitude/Amplitude-Java/tree/afb408aceb80fc9e7312bdc0915756e7ad76623a/src/main/java/com/amplitude)
(`Amplitude.java`, `Event`, `HttpCall`, `HttpTransport`, `Response`, and supporting
classes). The Java source revision is pinned here so compatibility changes can
be reviewed against a stable reference. Its MIT notice is included in
`src/amplitude/LICENSE-Amplitude-Java`.

The implementation lives in Rust in `statsig-rust::amplitude`. Only the Node
binding is added; there are no Java, Python, Go, PHP, Elixir, or C bindings for
Amplitude. Amplitude requests use their own API key and carry no Statsig headers.

```rust,no_run
use statsig_rust::{Amplitude, AmplitudeEvent};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let amplitude = Amplitude::get_instance(None); // None and Some("") share a client
amplitude.init("AMPLITUDE_API_KEY", None)?;
amplitude.set_event_upload_threshold(10);
let event = AmplitudeEvent::with_identity("Song Played", Some("user-123".into()), None)?;
amplitude.log_event(event).await?;
amplitude.flush_events_and_wait().await?;
amplitude.shutdown().await?;
# Ok(())
# }
```

## Client API

| Java | Rust | Node |
| --- | --- | --- |
| `getInstance()` / `getInstance(name)` | `get_instance(None)` / `get_instance(Some(name))` | `Amplitude.getInstance(name?)` |
| `init(key)` | `init(key, None)` | `init(key)` |
| `setServerUrl(url)` | `set_server_url(url)` | `setServerUrl(url)` |
| `setOptions(options)` | `set_options(Some(options))` | `setOptions(options)` |
| `useBatchMode(batch)` | `use_batch_mode(batch)` | `useBatchMode(batch)` |
| `setLogMode(mode)` / `setLogger(logger)` | `set_log_mode` / `set_logger` | `setLogMode` / `setLogger` |
| `setEventUploadThreshold(n)` | `set_event_upload_threshold(n)` | `setEventUploadThreshold(n)` |
| `setEventUploadPeriodMillis(ms)` | `set_event_upload_period_millis(ms)` | `setEventUploadPeriodMillis(ms)` |
| `setCallbacks(callbacks)` | `set_callbacks(Some(callback))` | `setCallbacks(callbacks)` |
| `setPlan(plan)` | `set_plan(Some(plan))` | `setPlan(plan)` |
| `setIngestionMetadata(metadata)` | `set_ingestion_metadata(Some(metadata))` | `setIngestionMetadata(metadata)` |
| `setProxy(proxy)` | `set_proxy(Some(proxy))` | `setProxy(proxy)` |
| `setFlushTimeout(ms)` | `set_flush_timeout(ms)` | `setFlushTimeout(ms)` |
| `setSendThreadPool(pool)` / `setRetryThreadPool(pool)` | `set_send_thread_pool` / `set_retry_thread_pool` | `setSendThreadPool(n)` / `setRetryThreadPool(n)` |
| `addEventMiddleware(middleware)` | `add_event_middleware(Arc::new(middleware))` | `addEventMiddleware(middleware)` |
| `logEvent(event[, callbacks, extra])` | `log_event` / `log_event_with` | `logEvent(event[, callbacks, extra])` |
| `flushEvents()` | `flush_events()` | `flushEvents()` |
| `shouldWait(event)` | `should_wait(&event)` | `shouldWait(event)` |
| `setRecordThrottledId(record)` | `set_record_throttled_id(record)` | `setRecordThrottledId(record)` |
| `shutdown()` | `shutdown().await` | `shutdown()` |

Named clients share configuration, buffers, callbacks, and middleware. The registry
retains them until shutdown, matching Java. `new()` remains available for independent
clients and compatibility with the original demo. `init` resets regular mode, and
a custom URL takes precedence over regular/batch URLs. Invalid URL prefixes are
ignored by `setServerUrl`, matching Java. Optional settings can be cleared with
Rust `None` or Node `null`.

`AmplitudeOptions` supports Java's `min_id_length` and custom headers. Its JSON
body contains only `min_id_length`; headers apply to the HTTP request. The original
demo's `init(key, {serverUrl, timeoutMs})` remains supported. Without these demo
options the network timeout is 10,000ms. `track(event)` also remains as an immediate,
awaitable convenience; Java-compatible buffering uses `logEvent`.

## Events, buffering, and callbacks

Every Java `Event` field is represented, including device/location/application
fields, advertising identifiers, groups and group properties, revenue, quantity,
product/currency, event/session/insert IDs, tracking plans, and ingestion metadata.
`timestamp` is the Java field name; the original demo's `time` remains an alias.
Event construction supplies a timestamp and UUID, with session ID -1, event ID 0,
and location coordinates 0. Null/omitted wire fields and revenue defaults match
Java. The library identifier identifies the actual Rust/Node SDK, rather than
claiming requests came from Java.

Properties recursively truncate strings to 1,000 UTF-16 code units and reject
objects with more than 1,024 keys. Event property, user property, group, and group
property setter/add helpers are available in Rust; Node accepts the equivalent
plain objects. Empty plan and ingestion metadata fields are omitted. Event-level
plans and ingestion metadata take precedence over client defaults.

The event buffer flushes at 10 events or 10,000ms by default. `logEvent` acknowledges
acceptance after middleware; delivery results arrive through client and event
callbacks `(event, status, message)`. Both callbacks run, in that order. Rust callbacks must not synchronously block
waiting for work on the same client. Dropped
middleware events trigger neither. `flush_events()` schedules delivery, matching
Java; `flush_events_and_wait()` is an additional Rust barrier. Node's `flushEvents`
returns a Promise for that barrier, including retries. Delivery failures are
reported through callbacks, while immediate `track` also rejects/returns an error.

## Transport and retries

Response status mapping, response-body status codes, invalid API-key exceptions,
and fallback to HTTP status for missing or malformed bodies follow the Java
implementation. Initial network failures become status 408. Flush execution timeout
is separate from network timeout, defaults to disabled (0), and applies to the
initial send rather than each retry.

Retryable responses are invalid requests, payload too large, rate limiting,
timeouts, and server failures. Retries group events by user/device pair and use
Java's 12 delays in milliseconds:
`100, 100, 200, 200, 400, 400, 800, 800, 1600, 1600, 3200, 3200`.

- HTTP 400 filters indexed invalid/missing fields; whole-payload missing fields and
  invalid singleton events are dropped. Remaining events retry with stable IDs.
- HTTP 429 drops identities with exceeded daily quotas, retries other identities,
  and optionally records only the IDs listed as throttled. `shouldWait` checks
  buffer pressure, the 16,000-event retry limit, and those recorded IDs.
- Repeated HTTP 413 matches Java's behavior: after a retry response, drop the latter
  half with callbacks and retry the first half. This deliberately does not resend
  both halves as an independent batch-splitting implementation would.
- Terminal statuses and exhaustion trigger callbacks once for each affected event.

Initial sends and retries have separate concurrency limits, defaulting to 20 and
10. Rust `AmplitudeThreadPool::with_handle` accepts a Tokio runtime handle and a
concurrency limit; Node accepts concurrency limits because Java ExecutorService
objects cannot cross the binding. Shared runtimes are not shut down by a client.
Proxy configuration is explicit; without a proxy, Amplitude ignores system proxy
settings, matching Java's `Proxy.NO_PROXY`.

## Middleware and lifecycle adaptations

Rust middleware receives a payload and an owned `AmplitudeMiddlewareNext`;
calling `next.run(payload).await` executes the remaining middleware. Returning
`None` drops the event. Node provides the Java-style `(payload, next)` continuation,
with an awaitable `next(payload)` across the native boundary:

```typescript
amplitude.addEventMiddleware(async (payload, next) => {
  payload.event.eventProperties = { requestId: payload.extra };
  await next(payload);
});
await amplitude.logEvent({ eventType: 'request', userId: 'user-123' }, { id: '123' });
await amplitude.flushEvents();
```

Callbacks can be functions or objects with `onLogEventServerResponse`. Loggers
provide `debug`, `warn`, and `error`, and modes are `DEBUG`, `WARN`, `ERROR`, and
`OFF`. Node methods that cross asynchronous middleware/network work return
Promises; setters remain synchronous and chainable. Node middleware passes JSON
values, not shared JavaScript object identity. Java long/int middleware fields
must remain safe integer-valued JavaScript numbers.

Shutdown stops new events and retries, invokes status-0 callbacks for buffered
work, waits for in-flight sends, releases Node callback references, and removes
named instances. Flush explicitly before shutdown to deliver buffered events.
This deterministically reports unsent events instead of preserving Java's silent
in-memory shutdown loss. Thread-safe instance lookup, single-use middleware
continuations, reference-counted throttling, and timer coordination also avoid
Java races and duplicate callbacks. Isolated UTF-16 surrogate boundaries use
Rust's replacement character when truncating. Invalid configuration, invalid properties, and middleware
exceptions return errors instead of reproducing Java's unchecked exceptions.

## Validation

`cargo test -p statsig-rust --test amplitude --locked` covers the Java-derived wire
format and defaults, buffers, middleware continuations, options/metadata/callbacks,
responses, invalid-event filtering, quota/throttle handling, payload reduction,
retry exhaustion, timeouts, and shutdown. The Node tests in
`statsig-node/src/__tests__/Amplitude.test.ts` exercise the compiled native addon,
not a JavaScript reimplementation or mock of the Rust client.
