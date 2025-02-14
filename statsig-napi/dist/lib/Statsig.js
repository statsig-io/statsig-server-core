"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Statsig = void 0;
const bindings_1 = require("./bindings");
const StatsigOptions_1 = require("./StatsigOptions");
class Statsig {
    constructor(sdkKey, options) {
        _initializeConsoleLogger(options === null || options === void 0 ? void 0 : options.outputLoggerLevel);
        this.__ref = (0, bindings_1.statsigCreate)(sdkKey, options === null || options === void 0 ? void 0 : options.__ref.refId);
    }
    initialize() {
        return (0, bindings_1.statsigInitialize)(this.__ref.refId);
    }
    shutdown() {
        return (0, bindings_1.statsigShutdown)(this.__ref.refId);
    }
    logEvent(user, eventName, value, metadata) {
        if (typeof value == 'number') {
            (0, bindings_1.statsigLogNumValueEvent)(this.__ref.refId, user.__ref.refId, eventName, value, metadata);
        }
        else {
            (0, bindings_1.statsigLogStringValueEvent)(this.__ref.refId, user.__ref.refId, eventName, value, metadata);
        }
    }
    checkGate(user, gateName, options) {
        return (0, bindings_1.statsigCheckGate)(this.__ref.refId, user.__ref.refId, gateName, options);
    }
    getFeatureGate(user, gateName, options) {
        return (0, bindings_1.statsigGetFeatureGate)(this.__ref.refId, user.__ref.refId, gateName, options);
    }
    manuallyLogGateExposure(user, gateName) {
        (0, bindings_1.statsigLogGateExposure)(this.__ref.refId, user.__ref.refId, gateName);
    }
    getDynamicConfig(user, dynamicConfigName, options) {
        const dynamicConfig = (0, bindings_1.statsigGetDynamicConfig)(this.__ref.refId, user.__ref.refId, dynamicConfigName, options);
        const value = JSON.parse(dynamicConfig.jsonValue);
        return {
            ...dynamicConfig,
            value,
            get: _makeTypedGet(value),
        };
    }
    manuallyLogDynamicConfigExposure(user, configName) {
        (0, bindings_1.statsigLogDynamicConfigExposure)(this.__ref.refId, user.__ref.refId, configName);
    }
    getExperiment(user, experimentName, options) {
        const experiment = (0, bindings_1.statsigGetExperiment)(this.__ref.refId, user.__ref.refId, experimentName, options);
        const value = JSON.parse(experiment.jsonValue);
        return {
            ...experiment,
            value,
            get: _makeTypedGet(value),
        };
    }
    manuallyLogExperimentExposure(user, gateName) {
        (0, bindings_1.statsigLogExperimentExposure)(this.__ref.refId, user.__ref.refId, gateName);
    }
    getLayer(user, layerName, options) {
        const layerJson = (0, bindings_1.statsigGetLayer)(this.__ref.refId, user.__ref.refId, layerName, options);
        const layer = JSON.parse(layerJson);
        const value = layer['__value'];
        return {
            ...layer,
            get: _makeTypedGet(value, (param) => {
                (0, bindings_1.statsigLogLayerParamExposure)(this.__ref.refId, layerJson, param);
            }),
        };
    }
    manuallyLogLayerParameterExposure(user, layerName, parameterName) {
        const layerJson = (0, bindings_1.statsigGetLayer)(this.__ref.refId, user.__ref.refId, layerName);
        (0, bindings_1.statsigLogLayerParamExposure)(this.__ref.refId, layerJson, parameterName);
    }
    getClientInitializeResponse(user, options) {
        return (0, bindings_1.statsigGetClientInitResponse)(this.__ref.refId, user.__ref.refId, options);
    }
}
exports.Statsig = Statsig;
function _isTypeMatch(a, b) {
    const typeOf = (x) => (Array.isArray(x) ? 'array' : typeof x);
    return typeOf(a) === typeOf(b);
}
function _makeTypedGet(value, exposeFunc) {
    return (param, fallback) => {
        var _a;
        const found = (_a = value === null || value === void 0 ? void 0 : value[param]) !== null && _a !== void 0 ? _a : null;
        if (found == null) {
            return (fallback !== null && fallback !== void 0 ? fallback : null);
        }
        if (fallback != null && !_isTypeMatch(found, fallback)) {
            return (fallback !== null && fallback !== void 0 ? fallback : null);
        }
        exposeFunc === null || exposeFunc === void 0 ? void 0 : exposeFunc(param);
        return found;
    };
}
// intentionally spaced for formatting
const DEBUG = ' DEBUG ';
const _INFO = '  INFO ';
const _WARN = '  WARN ';
const ERROR = ' ERROR ';
function _initializeConsoleLogger(level) {
    const initError = (0, bindings_1.consoleLoggerInit)((level !== null && level !== void 0 ? level : StatsigOptions_1.LogLevel.Error), (_, msg) => console.log('\x1b[32m%s\x1b[0m', DEBUG, msg), // Green text for DEBUG
    (_, msg) => console.info('\x1b[34m%s\x1b[0m', _INFO, msg), // Blue text for INFO
    (_, msg) => console.warn('\x1b[33m%s\x1b[0m', _WARN, msg), // Yellow text for WARN
    (_, msg) => console.error('\x1b[31m%s\x1b[0m', ERROR, msg));
    if (initError != null && level != StatsigOptions_1.LogLevel.None) {
        console.warn('\x1b[33m%s\x1b[0m', _WARN, `[Statsig]: ${initError}`);
    }
}
