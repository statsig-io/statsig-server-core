import {
  consoleLoggerInit,
  DynamicConfigNapi as DynamicConfig,
  ExperimentNapi as Experiment,
  FeatureGateNapi as FeatureGate,
  LayerNapi,
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

export type Layer = LayerNapi & {
  get: <T>(param: string, fallback: T) => T | null;
};

export enum LogLevel {
  None = 0,
  Error = 1,
  Warn = 2,
  Info = 3,
  Debug = 4,
}

export class StatsigOptions {
  readonly __ref: number;

  readonly outputLoggerLevel: LogLevel = LogLevel.Debug;

  constructor(
    outputLoggerLevel?: LogLevel,
    environment?: string | undefined | null,
    specsUrl?: string | undefined | null,
    logEventUrl?: string | undefined | null,
  ) {
    this.outputLoggerLevel = outputLoggerLevel ?? LogLevel.Error;
    this.__ref = statsigOptionsCreate(environment, specsUrl, logEventUrl);
  }
}

export class StatsigUser {
  readonly __ref: number;

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
      customIDs,
      email,
      ip,
      userAgent,
      country,
      locale,
      appVersion,
      custom,
      privateAttributes,
    );
  }
}

export class Statsig {
  readonly __ref: number;

  constructor(sdkKey: string, options?: StatsigOptions) {
    console.log('options', options);
    _initializeConsoleLogger(options?.outputLoggerLevel);

    this.__ref = statsigCreate(sdkKey, options?.__ref);
  }

  initialize(): Promise<void> {
    return statsigInitialize(this.__ref);
  }

  shutdown(): Promise<void> {
    return statsigShutdown(this.__ref);
  }

  logEvent(
    user: StatsigUser,
    eventName: string,
    value?: string | undefined | null,
    metadata?: Record<string, string> | undefined | null,
  ): void {
    statsigLogStringValueEvent(
      this.__ref,
      user.__ref,
      eventName,
      value,
      metadata,
    );
  }

  checkGate(user: StatsigUser, gateName: string): boolean {
    return statsigCheckGate(this.__ref, user.__ref, gateName);
  }

  getFeatureGate(user: StatsigUser, gateName: string): FeatureGate {
    return statsigGetFeatureGate(this.__ref, user.__ref, gateName);
  }

  getDynamicConfig(
    user: StatsigUser,
    dynamicConfigName: string,
  ): DynamicConfig {
    return statsigGetDynamicConfig(this.__ref, user.__ref, dynamicConfigName);
  }

  getExperiment(user: StatsigUser, experimentName: string): Experiment {
    return statsigGetExperiment(this.__ref, user.__ref, experimentName);
  }

  getLayer(user: StatsigUser, layerName: string): Layer {
    const layer = statsigGetLayer(this.__ref, user.__ref, layerName);
    return {
      ...layer,
      get: (_param, _fallback) => {
        // statsigLogLayerParamExposure(this.__ref, )
        return null; //todo
      },
    };
  }

  getClientInitializeResponse(user: StatsigUser): string {
    return statsigGetClientInitResponse(this.__ref, user.__ref);
  }
}

function _initializeConsoleLogger(level: LogLevel | undefined) {
  const errMessage = consoleLoggerInit(
    (level ?? LogLevel.Error) as any,
    (_, msg) => console.debug(msg),
    (_, msg) => console.info(msg),
    (_, msg) => console.warn(msg),
    (_, msg) => console.error(msg),
  );

  if (errMessage != null && level != LogLevel.None) {
    console.warn(errMessage);
  }
}
