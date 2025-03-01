/* auto-generated by NAPI-RS */
/* eslint-disable */
export declare class AutoReleasingStatsigOptionsRef {
  refId: string;
}

export declare class AutoReleasingStatsigRef {
  refId: string;
}

export declare class AutoReleasingStatsigUserRef {
  refId: string;
}

export interface AdapterResponse {
  result?: string;
  time?: number;
}

export interface ClientInitResponseOptions {
  hashAlgorithm?: string;
  clientSdkKey?: string;
  includeLocalOverrides?: boolean;
}

export declare function consoleLoggerInit(
  logLevel: LogLevel,
  debugLog: (message?: any, ...optionalParams: any[]) => void,
  infoLog: (message?: any, ...optionalParams: any[]) => void,
  warnLog: (message?: any, ...optionalParams: any[]) => void,
  errorLog: (message?: any, ...optionalParams: any[]) => void,
): string | null;

export interface DynamicConfigNapi {
  name: string;
  jsonValue: string;
  ruleID: string;
  idType: string;
  secondaryExposures?: Array<SecondaryExposure>;
  evaluationDetails?: EvaluationDetails;
}

export interface EvaluationDetails {
  reason: string;
  lcut?: number;
  receivedAt?: number;
}

export interface ExperimentNapi {
  name: string;
  jsonValue: string;
  ruleID: string;
  idType: string;
  groupName?: string;
  secondaryExposures?: Array<SecondaryExposure>;
  evaluationDetails?: EvaluationDetails;
}

export interface FeatureGateNapi {
  name: string;
  value: boolean;
  ruleID: string;
  idType: string;
  evaluationDetails?: EvaluationDetails;
}

export interface GetDynamicConfigOptions {
  disableExposureLogging: boolean;
}

export interface GetExperimentOptions {
  disableExposureLogging: boolean;
}

export interface GetFeatureGateOptions {
  disableExposureLogging: boolean;
}

export interface GetLayerOptions {
  disableExposureLogging: boolean;
}

export interface LayerNapi {
  name: string;
  ruleID: string;
  idType: string;
  groupName?: string;
  __jsonValue: string;
  jsonUser: string;
  evaluationDetails?: EvaluationDetails;
}

export declare const enum LogLevel {
  None = 0,
  Error = 1,
  Warn = 2,
  Info = 3,
  Debug = 4,
}

export interface SecondaryExposure {
  gate: string;
  gateValue: string;
  ruleId: string;
}

export interface SpecAdapterConfigNapi {
  adapterType: SpecsAdapterTypeNapi;
  specsUrl?: string;
  initTimeoutMs: number;
}

export declare const enum SpecsAdapterTypeNapi {
  NetworkHttp = 0,
  NetworkGrpcWebsocket = 1,
  DataStore = 2,
}

export declare function statsigCheckGate(
  statsigRef: string,
  userRef: string,
  gateName: string,
  options?: GetFeatureGateOptions | undefined | null,
): boolean;

export declare function statsigCreate(
  sdkKey: string,
  optionsRef?: string | undefined | null,
): AutoReleasingStatsigRef;

export declare function statsigGetClientInitResponse(
  statsigRef: string,
  userRef: string,
  options?: ClientInitResponseOptions | undefined | null,
): string;

export declare function statsigGetCurrentValues(
  statsigRef: string,
): string | null;

export declare function statsigGetDynamicConfig(
  statsigRef: string,
  userRef: string,
  dynamicConfigName: string,
  option?: GetDynamicConfigOptions | undefined | null,
): DynamicConfigNapi;

export declare function statsigGetExperiment(
  statsigRef: string,
  userRef: string,
  experimentName: string,
  option?: GetExperimentOptions | undefined | null,
): ExperimentNapi;

export declare function statsigGetFeatureGate(
  statsigRef: string,
  userRef: string,
  gateName: string,
  option?: GetFeatureGateOptions | undefined | null,
): FeatureGateNapi;

export declare function statsigGetLayer(
  statsigRef: string,
  userRef: string,
  layerName: string,
  option?: GetLayerOptions | undefined | null,
): string;

export declare function statsigInitialize(statsigRef: string): Promise<void>;

export declare function statsigLogDynamicConfigExposure(
  statsigRef: string,
  userRef: string,
  configName: string,
): void;

export declare function statsigLogExperimentExposure(
  statsigRef: string,
  userRef: string,
  experimentName: string,
): void;

export declare function statsigLogGateExposure(
  statsigRef: string,
  userRef: string,
  gateName: string,
): void;

export declare function statsigLogLayerParamExposure(
  statsigRef: string,
  layerData: string,
  paramName: string,
): void;

export declare function statsigLogNumValueEvent(
  statsigRef: string,
  userRef: string,
  eventName: string,
  value?: number | undefined | null,
  metadata?: Record<string, string> | undefined | null,
): void;

export declare function statsigLogStringValueEvent(
  statsigRef: string,
  userRef: string,
  eventName: string,
  value?: string | undefined | null,
  metadata?: Record<string, string> | undefined | null,
): void;

export declare function statsigOptionsCreate(
  environment?: string | undefined | null,
  dataStore?: object | undefined | null,
  specsUrl?: string | undefined | null,
  specsSyncIntervalMs?: number | undefined | null,
  logEventUrl?: string | undefined | null,
  eventLoggingMaxQueueSize?: number | undefined | null,
  eventLoggingFlushIntervalMs?: number | undefined | null,
  specAdaptersConfig?: Array<SpecAdapterConfigNapi> | undefined | null,
  observabilityClient?: object | undefined | null,
): AutoReleasingStatsigOptionsRef;

export declare function statsigShutdown(statsigRef: string): Promise<void>;

export declare function statsigUserCreate(
  userId?: string | undefined | null,
  customIdsJson?: string | undefined | null,
  email?: string | undefined | null,
  ip?: string | undefined | null,
  userAgent?: string | undefined | null,
  country?: string | undefined | null,
  locale?: string | undefined | null,
  appVersion?: string | undefined | null,
  customJson?: string | undefined | null,
  privateAttributesJson?: string | undefined | null,
): AutoReleasingStatsigUserRef;
