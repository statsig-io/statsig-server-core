import { DynamicConfig, Experiment, Layer } from '.';
import {
  AutoReleasingStatsigRef,
  ClientInitResponseOptions,
  FeatureGateNapi as FeatureGate,
  statsigCheckGate,
  statsigCreate,
  statsigGetClientInitResponse,
  statsigGetDynamicConfig,
  statsigGetExperiment,
  statsigGetFeatureGate,
  statsigGetLayer,
  statsigInitialize,
  statsigLogLayerParamExposure,
  statsigLogNumValueEvent,
  statsigLogStringValueEvent,
  statsigShutdown,
  consoleLoggerInit,
  statsigLogDynamicConfigExposure,
  statsigLogExperimentExposure,
  statsigLogGateExposure,
  GetExperimentOptions,
  GetFeatureGateOptions,
  GetLayerOptions,
} from './bindings';
import StatsigOptions, { LogLevel } from './StatsigOptions';
import StatsigUser from './StatsigUser';

export class Statsig {
  readonly __ref: AutoReleasingStatsigRef;

  constructor(sdkKey: string, options?: StatsigOptions) {
    _initializeConsoleLogger(options?.outputLoggerLevel);
    this.__ref = statsigCreate(sdkKey, options?.__ref.refId);
  }

  initialize(): Promise<void> {
    return statsigInitialize(this.__ref.refId);
  }

  shutdown(): Promise<void> {
    return statsigShutdown(this.__ref.refId);
  }

  logEvent(
    user: StatsigUser,
    eventName: string,
    value?: string | number | undefined | null,
    metadata?: Record<string, string> | undefined | null,
  ): void {
    if (typeof value == 'number') {
      statsigLogNumValueEvent(
        this.__ref.refId,
        user.__ref.refId,
        eventName,
        value,
        metadata,
      );
    } else {
      statsigLogStringValueEvent(
        this.__ref.refId,
        user.__ref.refId,
        eventName,
        value,
        metadata,
      );
    }
  }

  checkGate(
    user: StatsigUser,
    gateName: string,
    options?: GetFeatureGateOptions,
  ): boolean {
    return statsigCheckGate(
      this.__ref.refId,
      user.__ref.refId,
      gateName,
      options,
    );
  }

  getFeatureGate(
    user: StatsigUser,
    gateName: string,
    options?: GetFeatureGateOptions,
  ): FeatureGate {
    return statsigGetFeatureGate(
      this.__ref.refId,
      user.__ref.refId,
      gateName,
      options,
    );
  }

  manuallyLogGateExposure(user: StatsigUser, gateName: string) {
    statsigLogGateExposure(this.__ref.refId, user.__ref.refId, gateName);
  }

  getDynamicConfig(
    user: StatsigUser,
    dynamicConfigName: string,
    options?: GetExperimentOptions,
  ): DynamicConfig {
    const dynamicConfig = statsigGetDynamicConfig(
      this.__ref.refId,
      user.__ref.refId,
      dynamicConfigName,
      options,
    );

    const value = JSON.parse(dynamicConfig.jsonValue);
    return {
      ...dynamicConfig,
      value,
      get: _makeTypedGet(value),
    };
  }

  manuallyLogDynamicConfigExposure(user: StatsigUser, configName: string) {
    statsigLogDynamicConfigExposure(
      this.__ref.refId,
      user.__ref.refId,
      configName,
    );
  }

  getExperiment(
    user: StatsigUser,
    experimentName: string,
    options?: GetExperimentOptions,
  ): Experiment {
    const experiment = statsigGetExperiment(
      this.__ref.refId,
      user.__ref.refId,
      experimentName,
      options,
    );

    const value = JSON.parse(experiment.jsonValue);
    return {
      ...experiment,
      value,
      get: _makeTypedGet(value),
    };
  }

  manuallyLogExperimentExposure(user: StatsigUser, gateName: string) {
    statsigLogExperimentExposure(this.__ref.refId, user.__ref.refId, gateName);
  }

  getLayer(
    user: StatsigUser,
    layerName: string,
    options?: GetLayerOptions,
  ): Layer {
    const layerJson = statsigGetLayer(
      this.__ref.refId,
      user.__ref.refId,
      layerName,
      options,
    );

    const layer = JSON.parse(layerJson);
    const value = layer['__value'];
    return {
      ...layer,
      get: _makeTypedGet(value, (param: string) => {
        statsigLogLayerParamExposure(this.__ref.refId, layerJson, param);
      }),
    };
  }

  manuallyLogLayerParameterExposure(
    user: StatsigUser,
    layerName: string,
    parameterName: string,
  ) {
    const layerJson = statsigGetLayer(
      this.__ref.refId,
      user.__ref.refId,
      layerName,
    );

    statsigLogLayerParamExposure(this.__ref.refId, layerJson, parameterName);
  }

  getClientInitializeResponse(
    user: StatsigUser,
    options?: ClientInitResponseOptions,
  ): string {
    return statsigGetClientInitResponse(
      this.__ref.refId,
      user.__ref.refId,
      options,
    );
  }
}

// prettier-ignore
export type TypedReturn<T = unknown> = 
    T extends string ? string
  : T extends number ? number 
  : T extends boolean ? boolean
  : T extends Array<unknown> ? Array<unknown>
  : T extends object ? object
  : unknown;

export type TypedGet = <T = unknown>(
  key: string,
  fallback?: T,
) => TypedReturn<T>;

function _isTypeMatch<T>(a: unknown, b: unknown): a is T {
  const typeOf = (x: unknown) => (Array.isArray(x) ? 'array' : typeof x);
  return typeOf(a) === typeOf(b);
}

function _makeTypedGet(
  value: Record<string, unknown>,
  exposeFunc?: (param: string) => void,
): TypedGet {
  return <T = unknown>(param: string, fallback?: T) => {
    const found = value?.[param] ?? null;

    if (found == null) {
      return (fallback ?? null) as TypedReturn<T>;
    }

    if (fallback != null && !_isTypeMatch(found, fallback)) {
      return (fallback ?? null) as TypedReturn<T>;
    }

    exposeFunc?.(param);
    return found as TypedReturn<T>;
  };
}

// intentionally spaced for formatting
const DEBUG = ' DEBUG ';
const _INFO = '  INFO ';
const _WARN = '  WARN ';
const ERROR = ' ERROR ';

function _initializeConsoleLogger(level: LogLevel | undefined) {
  const initError = consoleLoggerInit(
    (level ?? LogLevel.Error) as any,
    (_, msg) => console.log('\x1b[32m%s\x1b[0m', DEBUG, msg), // Green text for DEBUG
    (_, msg) => console.info('\x1b[34m%s\x1b[0m', _INFO, msg), // Blue text for INFO
    (_, msg) => console.warn('\x1b[33m%s\x1b[0m', _WARN, msg), // Yellow text for WARN
    (_, msg) => console.error('\x1b[31m%s\x1b[0m', ERROR, msg), // Red text for ERROR
  );

  if (initError != null && level != LogLevel.None) {
    console.warn('\x1b[33m%s\x1b[0m', _WARN, `[Statsig]: ${initError}`);
  }
}
