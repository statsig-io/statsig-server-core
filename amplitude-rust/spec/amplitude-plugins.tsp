import "./amplitude-server-client.tsp";

namespace Amplitude.Analytics.Plugins;

/** Classification used to register plugins in a timeline. */
enum PluginType {
  before: 0,
  enrichment: 1,
  destination: 2,
  observe: 3,
}

/** Event subtype used for group property updates. */
model GroupIdentifyEvent extends Amplitude.Analytics.Event {
  event_type: string = "$groupidentify";
}

/** Event subtype used for user property updates. */
model IdentifyEvent extends Amplitude.Analytics.Event {
  event_type: string = "$identify";
}

/** Event subtype used to record revenue. */
model RevenueEvent extends Amplitude.Analytics.Event {
  event_type: string = "revenue_amount";
}
