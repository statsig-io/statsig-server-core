"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.LogLevel = void 0;
const bindings_1 = require("./bindings");
var LogLevel;
(function (LogLevel) {
    LogLevel[LogLevel["None"] = 0] = "None";
    LogLevel[LogLevel["Error"] = 1] = "Error";
    LogLevel[LogLevel["Warn"] = 2] = "Warn";
    LogLevel[LogLevel["Info"] = 3] = "Info";
    LogLevel[LogLevel["Debug"] = 4] = "Debug";
})(LogLevel || (exports.LogLevel = LogLevel = {}));
class StatsigOptions {
    constructor(optionArgs = {}) {
        var _a;
        this.outputLoggerLevel = LogLevel.Debug;
        this.outputLoggerLevel = (_a = optionArgs.outputLoggerLevel) !== null && _a !== void 0 ? _a : LogLevel.Error;
        const dataStoreWrapped = optionArgs.dataStore ? new WrappedDataStore(optionArgs.dataStore) : undefined;
        const obClient = optionArgs.observabilityClient ? new ObservabilityClientWrapped(optionArgs.observabilityClient) : undefined;
        this.__ref = (0, bindings_1.statsigOptionsCreate)(optionArgs.environment, dataStoreWrapped, optionArgs.specsUrl, optionArgs.specsSyncIntervalMs, optionArgs.logEventUrl, optionArgs.eventLoggingMaxQueueSize, optionArgs.eventLoggingFlushIntervalMs, optionArgs.specsAdapterConfig, obClient);
    }
}
exports.default = StatsigOptions;
class WrappedDataStore {
    constructor(client) {
        var _a;
        this.client = client;
        this.initialize = this.initialize.bind(this);
        this.get = this.get.bind(this);
        this.set = this.set.bind(this);
        this.shutdown = this.shutdown.bind(this);
        this.supportsPollingUpdatesFor = (_a = this.supportsPollingUpdatesFor) === null || _a === void 0 ? void 0 : _a.bind(this);
    }
    initialize(error) {
        return this.client.initialize();
    }
    get(error, key) {
        return this.client.get(key);
    }
    set(error, args) {
        let parsedArgs = JSON.parse(args);
        return this.client.set(parsedArgs.key, parsedArgs.value, parsedArgs.time);
    }
    shutdown(error) {
        return this.client.shutdown();
    }
    supportsPollingUpdatesFor(error, args) {
        var _a, _b;
        return (_b = (_a = this.client).supportsPollingUpdatesFor) === null || _b === void 0 ? void 0 : _b.call(_a, args);
    }
}
/**
 * Wrapper class to bridge arguments passed from rust side and interfaces
 */
class ObservabilityClientWrapped {
    constructor(client) {
        var _a;
        this.client = client;
        // This is needed otherwise, instance context will be lost
        this.init = this.init.bind(this);
        this.increment = this.increment.bind(this);
        this.gauge = this.gauge.bind(this);
        this.dist = this.dist.bind(this);
        this.should_enable_high_cardinality_for_this_tag = (_a = this.should_enable_high_cardinality_for_this_tag) === null || _a === void 0 ? void 0 : _a.bind(this);
    }
    init() {
        this.client.init();
    }
    increment(error, args) {
        let parsedArgs = JSON.parse(args);
        this.client.increment(parsedArgs.metric_name, parsedArgs.value, parsedArgs.tags);
    }
    gauge(error, args) {
        let parsedArgs = JSON.parse(args);
        this.client.gauge(parsedArgs.metric_name, parsedArgs.value, parsedArgs.tags);
    }
    dist(error, args) {
        let parsedArgs = JSON.parse(args);
        this.client.dist(parsedArgs.metric_name, parsedArgs.value, parsedArgs.tags);
    }
    should_enable_high_cardinality_for_this_tag(error, tag) {
        var _a, _b;
        (_b = (_a = this.client).should_enable_high_cardinality_for_this_tag) === null || _b === void 0 ? void 0 : _b.call(_a, tag);
    }
}
