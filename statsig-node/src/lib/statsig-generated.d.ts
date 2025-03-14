/* auto-generated by NAPI-RS */
/* eslint-disable */
export declare class DynamicConfig {
  name: string
  value: Record<string, any>
  ruleID: string
  idType: string
  getValue(param_name: string, fallback: boolean | number | string | object | Array<any> | null): any
  getRuleId(): string
  getIdType(): string
  getEvaluationDetails(): EvaluationDetails
  getSecondaryExposures(): Array<SecondaryExposure> | null
}

export declare class Experiment {
  name: string
  value: Record<string, any>
  ruleID: string
  idType: string
  groupName?: string
  getValue(param_name: string, fallback: boolean | number | string | object | Array<any> | null): any
  getRuleId(): string
  getIdType(): string
  getGroupName(): string | null
  getEvaluationDetails(): EvaluationDetails
  getSecondaryExposures(): Array<SecondaryExposure> | null
}

export declare class Layer {
  name: string
  ruleID: string
  groupName?: string
  allocatedExperimentName?: string
  getValue(param_name: string, fallback: boolean | number | string | object | Array<any> | null): any
  getRuleId(): string
  getGroupName(): string | null
  getEvaluationDetails(): EvaluationDetails
  getSecondaryExposures(): Array<SecondaryExposure> | null
}

export declare class Statsig {
  constructor(sdkKey: string, options?: StatsigOptions | undefined | null)
  initialize(): Promise<StatsigResult>
  shutdown(timeoutMs?: number | undefined | null): Promise<StatsigResult>
  flushEvents(): Promise<StatsigResult>
  logEvent(user: StatsigUser, eventName: string, value?: string | number | null, metadata?: Record<string, string> | undefined | null): void
  checkGate(user: StatsigUser, gateName: string, options?: FeatureGateEvaluationOptions | undefined | null): boolean
  getFeatureGate(user: StatsigUser, featureName: string, options?: FeatureGateEvaluationOptions | undefined | null): FeatureGate
  getFieldsNeededForGate(gateName: string): Array<string>
  getDynamicConfig(user: StatsigUser, configName: string, options?: DynamicConfigEvaluationOptions | undefined | null): DynamicConfig
  getFieldsNeededForDynamicConfig(configName: string): Array<string>
  getExperiment(user: StatsigUser, experimentName: string, options?: ExperimentEvaluationOptions | undefined | null): Experiment
  getFieldsNeededForExperiment(experimentName: string): Array<string>
  getLayer(user: StatsigUser, layerName: string, options?: LayerEvaluationOptions | undefined | null): Layer
  getFieldsNeededForLayer(layerName: string): Array<string>
  getClientInitializeResponse(user: StatsigUser, options?: ClientInitResponseOptions | undefined | null): string
  manuallyLogFeatureGateExposure(user: StatsigUser, gateName: string): void
  manuallyLogDynamicConfigExposure(user: StatsigUser, configName: string): void
  manuallyLogExperimentExposure(user: StatsigUser, experimentName: string): void
  manuallyLogLayerParamExposure(user: StatsigUser, layerName: string, paramName: string): void
  overrideGate(gateName: string, value: boolean, adapter?: OverrideAdapterType | undefined | null): void
  overrideDynamicConfig(configName: string, value: Record<string, any>, adapter?: OverrideAdapterType | undefined | null): void
  overrideExperiment(experimentName: string, value: Record<string, any>, adapter?: OverrideAdapterType | undefined | null): void
  overrideExperimentByGroupName(experimentName: string, groupName: string, adapter?: OverrideAdapterType | undefined | null): void
  overrideLayer(layerName: string, value: Record<string, any>, adapter?: OverrideAdapterType | undefined | null): void
}

export declare class StatsigUser {
  constructor(args: ({userID: string} | {customIDs: Record<string, string> }) & StatsigUserArgs)
  static withUserID(userId: string): StatsigUser
  static withCustomIDs(customIds: Record<string, string>): StatsigUser
  get customIDs(): Record<string, string> | null
  set customIDs(value: Record<string, string> | null)
  get custom(): Record<string, string> | null
  set custom(value: Record<string, string | number | boolean | Array<string | number | boolean>> | null)
  get privateAttributes(): Record<string, string> | null
  set privateAttributes(value: Record<string, string | number | boolean | Array<string | number | boolean>> | null)
  get userID(): string | null
  set userID(value: any)
  get email(): string | null
  set email(value: any)
  get ip(): string | null
  set ip(value: any)
  get userAgent(): string | null
  set userAgent(value: any)
  get country(): string | null
  set country(value: any)
  get locale(): string | null
  set locale(value: any)
  get appVersion(): string | null
  set appVersion(value: any)
}

export declare function __internal__testDataStore(store: DataStore, path: string, value: string): Promise<[DataStoreResponse | undefined | null, boolean]>

export declare function __internal__testObservabilityClient(client: ObservabilityClient, action: string, metricName: string, value: number, tags?: Record<string, string> | undefined | null): Promise<void>

export interface ClientInitResponseOptions {
  hashAlgorithm?: string
  clientSdkKey?: string
  includeLocalOverrides?: boolean
}

export interface DataStore {
  initialize?: () => Promise<void>
  shutdown?: () => Promise<void>
  get?: (key: string) => Promise<DataStoreResponse>
  set?: (key: string, value: string, time?: number) => Promise<void>
  supportPollingUpdatesFor?: (key: string) => Promise<boolean>
}

export interface DataStoreResponse {
  result?: string
  time?: number
}

export interface DynamicConfigEvaluationOptions {
  disableExposureLogging?: boolean
}

export interface EvaluationDetails {
  reason: string
  lcut?: number
  receivedAt?: number
}

export interface ExperimentEvaluationOptions {
  disableExposureLogging?: boolean
}

export interface FeatureGate {
  name: string
  value: boolean
  ruleID: string
  idType: string
}

export interface FeatureGateEvaluationOptions {
  disableExposureLogging?: boolean
}

export interface LayerEvaluationOptions {
  disableExposureLogging?: boolean
}

export interface ObservabilityClient {
  initialize?: () => void
  increment?: (metricName: string, value: number, tags: Record<string, string>) => void
  gauge?: (metricName: string, value: number, tags: Record<string, string>) => void
  dist?: (metricName: string, value: number, tags: Record<string, string>) => void
}

export interface OverrideAdapterConfig {
  adapterType: OverrideAdapterType
}

export declare const enum OverrideAdapterType {
  LocalOverride = 0
}

export interface SecondaryExposure {
  gate: string
  gateValue: string
  ruleId: string
}

export interface SpecAdapterConfig {
  adapterType: 'data_store' | 'network_grpc_websocket' | 'network_http'
  specsUrl?: string
  initTimeoutMs: number
}

export interface StatsigOptions {
  dataStore?: DataStore
  disableAllLogging?: boolean
  enableIdLists?: boolean
  enableUserAgentParsing?: boolean
  enableCountryLookup?: boolean
  environment?: string
  eventLoggingFlushIntervalMs?: number
  eventLoggingMaxQueueSize?: number
  fallbackToStatsigApi?: boolean
  idListsSyncIntervalMs?: number
  idListsUrl?: string
  initTimeoutMs?: number
  logEventUrl?: string
  observabilityClient?: ObservabilityClient
  outputLogLevel?: 'none' | 'debug' | 'info' | 'warn' | 'error'
  specAdaptersConfig?: Array<SpecAdapterConfig>
  specsUrl?: string
  specsSyncIntervalMs?: number
  serviceName?: string
  overrideAdapterConfig?: Array<OverrideAdapterConfig>
}

export interface StatsigResult {
  isSuccess: boolean
  error?: string
}

export interface StatsigUserArgs {
  userID?: string
  customIDs?: Record<string, string>
  email?: string
  ip?: string
  userAgent?: string
  country?: string
  locale?: string
  appVersion?: string
  custom?: Record<string, string | number | boolean | Array<string | number | boolean>>
  privateAttributes?: Record<string, string | number | boolean | Array<string | number | boolean>>
}
