import {
  SpecsAdapterTypeNapi as SpecsAdapterType,
  AutoReleasingStatsigOptionsRef,
  AutoReleasingStatsigRef,
  AutoReleasingStatsigUserRef,
  ClientInitResponseOptions,
  consoleLoggerInit,
  DynamicConfigNapi,
  ExperimentNapi,
  FeatureGateNapi as FeatureGate,
  LayerNapi,
  SpecAdapterConfigNapi as SpecAdapterConfig,
  statsigCheckGate,
  statsigCreate,
  statsigGetClientInitResponse,
  statsigGetDynamicConfig,
  statsigGetExperiment,
  statsigGetFeatureGate,
  statsigGetLayer,
  statsigInitialize,
  statsigLogLayerParamExposure,
  statsigLogStringValueEvent,
  statsigOptionsCreate,
  statsigShutdown,
  statsigUserCreate,
} from './bindings';

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

export type DynamicConfig = DynamicConfigNapi & {
  readonly value: Record<string, unknown>;
  readonly get: TypedGet;
};

export type Experiment = ExperimentNapi & {
  readonly value: Record<string, unknown>;
  readonly get: TypedGet;
};

export type Layer = LayerNapi & {
  readonly get: TypedGet;
};

export { SpecAdapterConfig };

export enum LogLevel {
  None = 0,
  Error = 1,
  Warn = 2,
  Info = 3,
  Debug = 4,
}

export enum SpecAdapterType {
  NetworkHttp = 0,
  NetworkGrpcWebsocket = 1,
}

export class StatsigOptions {
  readonly __ref: AutoReleasingStatsigOptionsRef;

  readonly outputLoggerLevel: LogLevel = LogLevel.Debug;

  constructor(
    outputLoggerLevel?: LogLevel,
    environment?: string | undefined | null,
    specsUrl?: string | undefined | null,
    specsSyncIntervalMs?: number | undefined | null,
    logEventUrl?: string | undefined | null,
    eventLoggingMaxQueueSize?: number | undefined | null,
    eventLoggingFlushIntervalMs?: number | undefined | null,
    specs_adapter_configs?: Array<SpecAdapterConfig> | undefined | null,
  ) {
    this.outputLoggerLevel = outputLoggerLevel ?? LogLevel.Error;
    this.__ref = statsigOptionsCreate(
      environment,
      specsUrl,
      specsSyncIntervalMs,
      logEventUrl,
      eventLoggingMaxQueueSize,
      eventLoggingFlushIntervalMs,
      specs_adapter_configs,
    );
  }
}

export class StatsigUser {
  readonly __ref: AutoReleasingStatsigUserRef;

  constructor(
    userID: string,
    customIDs: Record<string, string>,
    email?: string | undefined | null,
    ip?: string | undefined | null,
    userAgent?: string | undefined | null,
    country?: string | undefined | null,
    locale?: string | undefined | null,
    appVersion?: string | undefined | null,
    custom?: Record<string, string> | undefined | null,
    privateAttributes?: Record<string, string> | undefined | null,
  ) {
    this.__ref = statsigUserCreate(
      userID,
      JSON.stringify(customIDs),
      email,
      ip,
      userAgent,
      country,
      locale,
      appVersion,
      custom ? JSON.stringify(custom) : null,
      privateAttributes ? JSON.stringify(privateAttributes) : null,
    );
  }
}

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
    value?: string | undefined | null,
    metadata?: Record<string, string> | undefined | null,
  ): void {
    statsigLogStringValueEvent(
      this.__ref.refId,
      user.__ref.refId,
      eventName,
      value,
      metadata,
    );
  }

  checkGate(user: StatsigUser, gateName: string): boolean {
    return statsigCheckGate(this.__ref.refId, user.__ref.refId, gateName);
  }

  getFeatureGate(user: StatsigUser, gateName: string): FeatureGate {
    return statsigGetFeatureGate(this.__ref.refId, user.__ref.refId, gateName);
  }

  getDynamicConfig(
    user: StatsigUser,
    dynamicConfigName: string,
  ): DynamicConfig {
    const dynamicConfig = statsigGetDynamicConfig(
      this.__ref.refId,
      user.__ref.refId,
      dynamicConfigName,
    );

    const value = JSON.parse(dynamicConfig.jsonValue);
    return {
      ...dynamicConfig,
      value,
      get: _makeTypedGet(value),
    };
  }

  getExperiment(user: StatsigUser, experimentName: string): Experiment {
    const experiment = statsigGetExperiment(
      this.__ref.refId,
      user.__ref.refId,
      experimentName,
    );

    const value = JSON.parse(experiment.jsonValue);
    return {
      ...experiment,
      value,
      get: _makeTypedGet(value),
    };
  }

  getLayer(user: StatsigUser, layerName: string): Layer {
    const layerJson = statsigGetLayer(
      this.__ref.refId,
      user.__ref.refId,
      layerName,
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
