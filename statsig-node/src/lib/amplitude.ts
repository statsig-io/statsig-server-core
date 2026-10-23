import {
  AmplitudeCallbacks,
  AmplitudeEvent,
  AmplitudeIngestionMetadata,
  AmplitudeLogger,
  AmplitudeMiddleware,
  AmplitudeNapi,
  AmplitudeOptions,
  AmplitudePlan,
  ProxyConfig,
} from './statsig-generated';

/** Node binding for the Rust implementation of Amplitude-Java. */
export class Amplitude {
  private static readonly instances = new Map<string, Amplitude>();
  private native: AmplitudeNapi;

  constructor() {
    this.native = new AmplitudeNapi();
  }

  static getInstance(name?: string | null): Amplitude {
    const key = name ?? '';
    const existing = this.instances.get(key);
    if (existing) return existing;
    const instance = new Amplitude();
    instance.native = AmplitudeNapi.getInstance(key);
    this.instances.set(key, instance);
    return instance;
  }

  init(apiKey: string, options?: AmplitudeOptions): void {
    this.native.init(apiKey, options);
  }

  setServerUrl(url: string): this { this.native.setServerUrl(url); return this; }
  setOptions(options: AmplitudeOptions | null): this { this.native.setOptions(options); return this; }
  useBatchMode(batch: boolean): this { this.native.useBatchMode(batch); return this; }
  setLogMode(mode: 'DEBUG' | 'WARN' | 'ERROR' | 'OFF'): this { this.native.setLogMode(mode); return this; }
  setLogger(logger: AmplitudeLogger): this { this.native.setLogger(logger); return this; }
  setEventUploadThreshold(threshold: number): this { this.native.setEventUploadThreshold(threshold); return this; }
  setEventUploadPeriodMillis(period: number): this { this.native.setEventUploadPeriodMillis(period); return this; }
  setPlan(plan: AmplitudePlan | null): this { this.native.setPlan(plan); return this; }
  setIngestionMetadata(metadata: AmplitudeIngestionMetadata | null): this { this.native.setIngestionMetadata(metadata); return this; }
  setProxy(proxy: ProxyConfig | null): this { this.native.setProxy(proxy); return this; }
  setFlushTimeout(timeout: number): this { this.native.setFlushTimeout(timeout); return this; }
  setSendThreadPool(concurrency: number): this { this.native.setSendThreadPool(concurrency); return this; }
  setRetryThreadPool(concurrency: number): this { this.native.setRetryThreadPool(concurrency); return this; }
  setRecordThrottledId(record: boolean): this { this.native.setRecordThrottledId(record); return this; }

  setCallbacks(callbacks: AmplitudeCallbacks | null): this {
    this.native.setCallbacks(callbacks == null ? null : responseCallback(callbacks));
    return this;
  }

  addEventMiddleware(middleware: AmplitudeMiddleware): this {
    this.native.addEventMiddleware(async (payload, next) => {
      let forwarded: ReturnType<typeof next.run> | undefined;
      await middleware(payload, (updated = payload) => {
        if (forwarded) throw new Error('Middleware next can only be called once');
        forwarded = next.run(updated);
        return forwarded;
      });
      return forwarded ? await forwarded : null;
    });
    return this;
  }

  logEvent(event: AmplitudeEvent): Promise<void>;
  logEvent(event: AmplitudeEvent, extra: Record<string, unknown>): Promise<void>;
  logEvent(event: AmplitudeEvent, callbacks: AmplitudeCallbacks | null, extra?: Record<string, unknown>): Promise<void>;
  logEvent(
    event: AmplitudeEvent,
    callbacksOrExtra?: AmplitudeCallbacks | Record<string, unknown> | null,
    extra?: Record<string, unknown>,
  ): Promise<void> {
    const hasCallback = typeof callbacksOrExtra === 'function' ||
      (callbacksOrExtra != null && typeof callbacksOrExtra.onLogEventServerResponse === 'function');
    return this.native.logEvent(
      event,
      hasCallback ? responseCallback(callbacksOrExtra as AmplitudeCallbacks) : null,
      hasCallback || callbacksOrExtra == null ? extra : callbacksOrExtra,
    );
  }

  shouldWait(event: AmplitudeEvent): boolean { return this.native.shouldWait(event); }
  flushEvents(): Promise<void> { return this.native.flushEvents(); }
  track(event: AmplitudeEvent): Promise<void> { return this.native.track(event); }

  async shutdown(): Promise<void> {
    await this.native.shutdown();
    for (const [name, instance] of Amplitude.instances) {
      if (instance === this) Amplitude.instances.delete(name);
    }
  }
}

function responseCallback(callbacks: AmplitudeCallbacks): (event: AmplitudeEvent, status: number, message: string) => void {
  return typeof callbacks === 'function'
    ? callbacks
    : callbacks.onLogEventServerResponse.bind(callbacks);
}
