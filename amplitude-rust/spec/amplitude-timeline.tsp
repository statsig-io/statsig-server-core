import "./amplitude-plugins.tsp";

namespace Amplitude.Analytics;

/** Stages with a registration list and lock in a Timeline. */
alias TimelineStage = Plugins.PluginType.before | Plugins.PluginType.enrichment | Plugins.PluginType.destination;
