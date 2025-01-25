import { ObservabilityClient } from '../../build/index.js';

export class MockObservabilityClient implements ObservabilityClient {
  static verbose = false;

  initialize() {
    if (MockObservabilityClient.verbose) {
      console.log('MockObservabilityClient: initialize');
    }
  }

  increment(metricName: string, value: number, tags: Record<string, string>) {
    if (MockObservabilityClient.verbose) {
      console.log(
        'MockObservabilityClient: increment',
        metricName,
        value,
        tags,
      );
    }
  }

  gauge(metricName: string, value: number, tags: Record<string, string>) {
    if (MockObservabilityClient.verbose) {
      console.log('MockObservabilityClient: gauge', metricName, value, tags);
    }
  }

  dist(metricName: string, value: number, tags: Record<string, string>) {
    if (MockObservabilityClient.verbose) {
      console.log('MockObservabilityClient: dist', metricName, value, tags);
    }
  }
}
