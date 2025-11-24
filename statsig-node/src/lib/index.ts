import { HttpsProxyAgent } from 'https-proxy-agent';
import nodeFetch from 'node-fetch';

import {
  startStatsigConsoleCapture,
  stopStatsigConsoleCapture,
} from './console_capture';
import { ErrorBoundary } from './error_boundary';
import {
  DynamicConfig,
  Experiment,
  Layer,
  ParameterStore,
  SdkEvent,
  StatsigNapiInternal,
  StatsigOptions,
  StatsigResult,
  StatsigUser,
} from './statsig-generated';

export * from './statsig-generated';

const inspectSym = Symbol.for('nodejs.util.inspect.custom');

// @ts-expect-error - prototype assignment
StatsigUser.prototype[inspectSym] = function () {
  return this.toJSON();
};
// @ts-expect-error - prototype assignment
Experiment.prototype[inspectSym] = function () {
  return this.toJSON();
};
// @ts-expect-error - prototype assignment
DynamicConfig.prototype[inspectSym] = function () {
  return this.toJSON();
};
// @ts-expect-error - prototype assignment
Layer.prototype[inspectSym] = function () {
  return this.toJSON();
};
// @ts-expect-error - prototype assignment
ParameterStore.prototype[inspectSym] = function () {
  return this.toJSON();
};

export { StatsigUser, Experiment, DynamicConfig, Layer, ParameterStore };

function createProxyAgent(options?: StatsigOptions) {
  const proxy = options?.proxyConfig;
  if (proxy?.proxyHost && proxy?.proxyProtocol) {
    const protocol = proxy.proxyProtocol;
    const host = proxy.proxyHost;
    const port = proxy.proxyPort ? `:${proxy.proxyPort}` : '';
    const auth = proxy.proxyAuth ? `${proxy.proxyAuth}@` : '';
    const proxyUrl = `${protocol}://${auth}${host}${port}`;

    if (protocol === 'http' || protocol === 'https') {
      return new HttpsProxyAgent(proxyUrl);
    }
  }
  return undefined; // node-fetch agent parameter takes in undefined type instead of null
}

function createFetchFunc(options?: StatsigOptions) {
  const proxyAgent = createProxyAgent(options);

  return async (
    method: string,
    url: string,
    headers: Record<string, string>,
    body?: Uint8Array,
  ) => {
    try {
      const res = await nodeFetch(url, {
        method,
        headers: {
          ...headers,
          'Accept-Encoding': 'gzip, deflate, br',
        },
        body: body ? Buffer.from(body) : undefined,
        agent: proxyAgent,
      });

      const data = await res.arrayBuffer();
      const resHeaders = Object.fromEntries(res.headers.entries()) as Record<
        string,
        string
      >;

      return {
        status: res.status,
        data: Array.from(new Uint8Array(data)),
        headers: resHeaders,
      };
    } catch (err: any) {
      return {
        status: 0,
        error: 'message' in err ? err.message : 'Unknown Node Fetch Error',
      };
    }
  };
}

export class Statsig extends StatsigNapiInternal {
  private static _sharedInstance: Statsig | null = null;

  public static shared(): Statsig {
    if (!Statsig.hasShared()) {
      console.warn(
        '[Statsig] No shared instance has been created yet. Call newShared() before using it. Returning an invalid instance',
      );
      return _createErrorInstance();
    }
    return Statsig._sharedInstance!;
  }

  public static hasShared(): boolean {
    return Statsig._sharedInstance !== null;
  }

  public static newShared(sdkKey: string, options?: StatsigOptions): Statsig {
    if (Statsig.hasShared()) {
      console.warn(
        '[Statsig] Shared instance has been created, call removeSharedInstance() if you want to create another one. ' +
          'Returning an invalid instance',
      );
      return _createErrorInstance();
    }

    Statsig._sharedInstance = new Statsig(sdkKey, options);
    return Statsig._sharedInstance;
  }

  public static removeSharedInstance() {
    Statsig._sharedInstance = null;
  }

  constructor(sdkKey: string, options?: StatsigOptions) {
    const fetchFunc = createFetchFunc(options);
    super(fetchFunc, sdkKey, options);

    ErrorBoundary.wrap(this);
    if (options?.consoleCaptureOptions?.enabled) {
      startStatsigConsoleCapture(sdkKey, options.consoleCaptureOptions);
    }
  }

  public stopConsoleCapture() {
    stopStatsigConsoleCapture();
  }

  public async shutdown(timeout_ms?: number): Promise<StatsigResult> {
    stopStatsigConsoleCapture();
    return super.shutdown(timeout_ms);
  }

  public subscribe(
    eventName: SdkEvent,
    callback: (event: any) => void,
  ): string {
    return this.__INTERNAL_subscribe(eventName, (raw) => {
      try {
        callback(JSON.parse(raw));
      } catch (error) {
        console.error(`Error parsing event: ${error}`);
      }
    });
  }
}

function _createErrorInstance(): Statsig {
  let dummyInstance = new Statsig('INVALID-KEY');
  dummyInstance.shutdown();
  return dummyInstance;
}
