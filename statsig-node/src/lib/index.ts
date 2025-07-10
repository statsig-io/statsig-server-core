import { HttpsProxyAgent } from 'https-proxy-agent';
import nodeFetch from 'node-fetch';

import { ErrorBoundary } from './error_boundary';
import { StatsigNapiInternal, StatsigOptions } from './statsig-generated';

export * from './statsig-generated';

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
        headers,
        body: body ? Buffer.from(body) : undefined,
        agent: proxyAgent,
      });

      const data = await res.arrayBuffer();

      return {
        status: res.status,
        data: Array.from(new Uint8Array(data)),
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
      return Statsig._createErrorInstance();
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
      return Statsig._createErrorInstance();
    }

    Statsig._sharedInstance = new Statsig(sdkKey, options);
    return Statsig._sharedInstance;
  }

  public static removeSharedInstance() {
    Statsig._sharedInstance = null;
  }

  private static _createErrorInstance(): Statsig {
    let dummyInstance = new Statsig('INVALID-KEY');
    dummyInstance.shutdown();
    return dummyInstance;
  }

  constructor(sdkKey: string, options?: StatsigOptions) {
    const fetchFunc = createFetchFunc(options);
    super(fetchFunc, sdkKey, options);

    ErrorBoundary.wrap(this);
  }
}
