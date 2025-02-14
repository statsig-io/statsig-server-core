import { AutoReleasingStatsigOptionsRef, SpecAdapterConfigNapi as SpecAdapterConfig } from './bindings';
import { IDataStore } from './IDataStore';
export type StatsigOptionArgs = Partial<{
    outputLoggerLevel?: LogLevel;
    environment?: string | undefined | null;
    specsUrl?: string | undefined | null;
    specsSyncIntervalMs?: number | undefined | null;
    logEventUrl?: string | undefined | null;
    eventLoggingMaxQueueSize?: number | undefined | null;
    eventLoggingFlushIntervalMs?: number | undefined | null;
    dataStore?: IDataStore | undefined | null;
    specsAdapterConfig?: Array<SpecAdapterConfig> | undefined | null;
    observabilityClient?: IObservabilityClient | undefined | null;
}>;
export declare enum LogLevel {
    None = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4
}
export default class StatsigOptions {
    readonly __ref: AutoReleasingStatsigOptionsRef;
    readonly outputLoggerLevel: LogLevel;
    constructor(optionArgs?: StatsigOptionArgs);
}
export interface IObservabilityClient {
    init(): void;
    increment(metricName: string, value: number, tags: Record<string, any>): void;
    gauge(metricName: string, value: number, tags: Record<string, any>): void;
    dist(metricName: string, value: number, tags: Record<string, any>): void;
    should_enable_high_cardinality_for_this_tag?(tag: string): void;
}
