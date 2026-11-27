namespace Amplitude.Analytics;

union JsonValue {
  null,
  boolean,
  numeric,
  string,
  JsonValue[],
  JsonObject,
}

model JsonObject is Record<JsonValue>;

model Plan {
  branch?: string;
  source?: string;
  version?: string;
  version_id?: string;
}

model IngestionMetadata {
  source_name?: string;
  source_version?: string;
}

enum ServerZone {
  us,
  eu,
}

enum LogLevel {
  None: 0,
  Error: 1,
  Warn: 2,
  Verbose: 3,
  Debug: 4,
}

/** Server client initialization fields. */
model Config {
  /** Project API key, passed as a separate constructor argument. */
  api_key: string;
  /** how long to hold events in queue until flush */
  flush_interval_millis?: float64;
  /** number of retries on flushing before giving up (only retries on "retryable errors") */
  flush_max_retries?: float64;
  /** max event queue size before flush */
  flush_queue_size?: float64;
  /** name to distinguish client instances */
  instance_name?: string;
  /** log level of SDK */
  log_level?: LogLevel;
  /** when optOut is set, track nothing (client-only) */
  opt_out?: boolean = false;
  /** tracking plan metadata */
  plan?: Plan;
  /** Default ingestion metadata for events that omit it. */
  ingestion_metadata?: IngestionMetadata;
  /** send events to another server. */
  server_url?: string;
  /** data center to send events to (US | EU) */
  server_zone?: ServerZone = ServerZone.us;
  /** use batch mode */
  use_batch?: boolean = false;
  /** Server HTTP request timeout in milliseconds. */
  @minValue(1)
  timeout_ms?: uint64 = 10000;
}

model EventOptions {
  user_id?: string;
  device_id?: string;
  /** Event time in milliseconds since the Unix epoch. */
  time?: int64;
  app_version?: string;
  platform?: string;
  os_name?: string;
  os_version?: string;
  device_brand?: string;
  device_manufacturer?: string;
  device_model?: string;
  carrier?: string;
  country?: string;
  region?: string;
  city?: string;
  dma?: string;
  language?: string;
  price?: float64;
  quantity?: int64;
  revenue?: float64;
  product_id?: string;
  revenue_type?: string;
  currency?: string;
  location_lat?: float64;
  location_lng?: float64;
  ip?: string;
  idfa?: string;
  idfv?: string;
  adid?: string;
  android_id?: string;
  event_id?: int64;
  session_id?: int64;
  insert_id?: string;
  plan?: Plan;
  ingestion_metadata?: IngestionMetadata;
  partner_id?: string;
  version_name?: string;
  user_agent?: string;
}

union GroupName {
  single: string,
  multiple: string[],
}

model Event {
  event_type: string;
  options: EventOptions;
  event_properties?: JsonObject;
  user_properties?: JsonObject;
  groups?: Record<GroupName>;
  group_properties?: JsonObject;
  library?: string;
}

model Result {
  event: Event;
  code: float64;
  message: string;
}

model PropertyValue {
  key: string;
  value: JsonValue;
}

model PropertyIncrement {
  key: string;
  value: float64;
}

@discriminated(#{ envelope: "none", discriminatorPropertyName: "operation" })
union IdentifyOperation {
  set: PropertyValue,
  set_once: PropertyValue,
  append: PropertyValue,
  prepend: PropertyValue,
  pre_insert: PropertyValue,
  post_insert: PropertyValue,
  remove: PropertyValue,
  add: PropertyIncrement,
  unset: { key: string },
  clear_all: {},
}

alias Identify = IdentifyOperation[];
