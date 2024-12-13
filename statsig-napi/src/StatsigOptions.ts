import {
  SpecsAdapterTypeNapi as SpecsAdapterType,
  AutoReleasingStatsigOptionsRef,
  FeatureGateNapi as FeatureGate,
  SpecAdapterConfigNapi as SpecAdapterConfig,
  statsigOptionsCreate,
} from './bindings';
import {
  IDataStore,
  getDataStoreKey,
  DataStoreKeyPath,
  AdapterResponse
} from './IDataStore'

export type StatsigOptionArgs = Partial<{
  outputLoggerLevel?: LogLevel,
    environment?: string | undefined | null,
    specsUrl?: string | undefined | null,
    specsSyncIntervalMs?: number | undefined | null,
    logEventUrl?: string | undefined | null,
    eventLoggingMaxQueueSize?: number | undefined | null,
    eventLoggingFlushIntervalMs?: number | undefined | null,
    dataStore?: IDataStore | undefined | null,
    specsAdapterConfig?: Array<SpecAdapterConfig> | undefined | null,
    observabilityClient?: IObservabilityClient | undefined | null,
}>

export enum LogLevel {
  None = 0,
  Error = 1,
  Warn = 2,
  Info = 3,
  Debug = 4,
}

export default class StatsigOptions {
  readonly __ref: AutoReleasingStatsigOptionsRef;

  readonly outputLoggerLevel: LogLevel = LogLevel.Debug;

  constructor(
    optionArgs: StatsigOptionArgs = {}
  ) {
    this.outputLoggerLevel = optionArgs.outputLoggerLevel ?? LogLevel.Error;
    const dataStoreWrapped = optionArgs.dataStore ? new WrappedDataStore(optionArgs.dataStore) : undefined;
    const obClient = optionArgs.observabilityClient ? new ObservabilityClientWrapped(optionArgs.observabilityClient) : undefined;
    this.__ref = statsigOptionsCreate(
      optionArgs.environment,
      dataStoreWrapped,
      optionArgs.specsUrl,
      optionArgs.specsSyncIntervalMs,
      optionArgs.logEventUrl,
      optionArgs.eventLoggingMaxQueueSize,
      optionArgs.eventLoggingFlushIntervalMs,
      optionArgs.specsAdapterConfig,
      obClient
    );
  }
}

class WrappedDataStore {
  constructor(private client: IDataStore) {
    this.initialize = this.initialize.bind(this)
    this.get = this.get.bind(this)
    this.set = this.set.bind(this)
    this.shutdown = this.shutdown.bind(this)
    this.supportsPollingUpdatesFor = this.supportsPollingUpdatesFor?.bind(this)
  }

  initialize(error: Error | undefined | null): Promise<void>  {
    return this.client.initialize();
  }

  get(error: Error | undefined | null, key: string): Promise<AdapterResponse> {
    return this.client.get(key)
  }

  set(error: Error | undefined | null, args: string): Promise<void> {
    let parsedArgs = JSON.parse(args);
    return this.client.set(parsedArgs.key, parsedArgs.value, parsedArgs.time);
  }

  shutdown(error: Error | undefined | null): Promise<void> {
    return this.client.shutdown();
  }

  supportsPollingUpdatesFor(error: Error | undefined | null, args: String): boolean | undefined {
    return this.client.supportsPollingUpdatesFor?.(args as DataStoreKeyPath)
  }
}

export interface IObservabilityClient {
  init(): void;
  increment(metricName: string, value: number, tags: Record<string, any>): void;
  gauge(metricName: string, value: number, tags: Record<string, any>): void;
  dist(metricName: string, value: number, tags: Record<string, any>): void;
  should_enable_high_cardinality_for_this_tag?(tag: string): void;
}

/**
 * Wrapper class to bridge arguments passed from rust side and interfaces
 */
class ObservabilityClientWrapped {
  private client: IObservabilityClient
  constructor(client: IObservabilityClient){
    this.client = client;
    // This is needed otherwise, instance context will be lost
    this.init = this.init.bind(this);
    this.increment = this.increment.bind(this);
    this.gauge = this.gauge.bind(this);
    this.dist = this.dist.bind(this);
    this.should_enable_high_cardinality_for_this_tag = this.should_enable_high_cardinality_for_this_tag?.bind(this);
  }

  init(): void {
    this.client.init();
  }

  increment(error: undefined | null | Error, args: string): void {
    let parsedArgs = JSON.parse(args);
    this.client.increment(parsedArgs.metric_name, parsedArgs.value, parsedArgs.tags);
  }

  gauge(error: undefined | null | Error, args: string): void {
    let parsedArgs = JSON.parse(args);
    this.client.gauge(parsedArgs.metric_name, parsedArgs.value, parsedArgs.tags);
  }
  
  dist(error: undefined | null | Error, args: string): void {
    let parsedArgs = JSON.parse(args);
    this.client.dist(parsedArgs.metric_name, parsedArgs.value, parsedArgs.tags);
  }

  should_enable_high_cardinality_for_this_tag?(error: undefined | null | Error, tag: string): void {
    this.client.should_enable_high_cardinality_for_this_tag?.(tag);
  }
}
