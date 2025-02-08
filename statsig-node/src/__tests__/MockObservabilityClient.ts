import { ObservabilityClient } from '../../build/index.js';

export class MockObservabilityClient implements ObservabilityClient {
  verbose = false;

  readonly initialize = () => {
    if (this.verbose) {
      console.log('MockObservabilityClient: initialize');
    }
    return Promise.resolve();
  };

  readonly increment = (
    metricName: string,
    value: number,
    tags: Record<string, string>,
  ) => {
    if (this.verbose) {
      console.log(
        'MockObservabilityClient: increment',
        metricName,
        value,
        tags,
      );
    }
    return Promise.resolve();
  };

  readonly gauge = (
    metricName: string,
    value: number,
    tags: Record<string, string>,
  ) => {
    if (this.verbose) {
      console.log('MockObservabilityClient: gauge', metricName, value, tags);
    }
    return Promise.resolve();
  };

  readonly dist = (
    metricName: string,
    value: number,
    tags: Record<string, string>,
  ) => {
    if (this.verbose) {
      console.log('MockObservabilityClient: dist', metricName, value, tags);
    }
  };
}
