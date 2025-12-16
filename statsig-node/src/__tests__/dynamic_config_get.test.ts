import { DynamicConfig } from '../lib/statsig_types';

interface SettingsMap {
  [key: string]: string | number | boolean | SettingsMap | undefined;
}

// fake use cases copied from our services
type FakeSettingsType = Record<
  string,
  string | number | boolean | SettingsMap | undefined
>;

// fake use cases2
type BigQueryTablesConfig = {
  FAKE_BQ_CONFIG: string | null;
};

// fake use cases3
type RetryOptions = {
  retryDelayMultiplier?: number;
  totalTimeout?: number;
  maxRetryDelay?: number;
  autoRetry?: boolean;
  maxRetries?: number;
  missingKey?: number;
}

describe('DynamicConfig.get typing regressions', () => {
  it('allows assigning RetryOptions / undefined from get()', () => {
    const config = new DynamicConfig(
      'cfg',
      JSON.stringify({ value: { retryDelayMultiplier: 1.5, totalTimeout: 10000, maxRetryDelay: 1000, autoRetry: true, maxRetries: 3 } }),
    );
    const returnVal: RetryOptions = {
      retryDelayMultiplier: config.get('retryDelayMultiplier', undefined),
      totalTimeout: config.get('totalTimeout', undefined),
      maxRetryDelay: config.get('maxRetryDelay', undefined),
      autoRetry: config.get('autoRetry', undefined),
      maxRetries: config.get('maxRetries', undefined),
      missingKey: config.get('missingKey', undefined),
    };
    expect(returnVal.retryDelayMultiplier).toBe(1.5);
    expect(returnVal.totalTimeout).toBe(10000);
    expect(returnVal.maxRetryDelay).toBe(1000);
    expect(returnVal.autoRetry).toBe(true);
    expect(returnVal.maxRetries).toBe(3);
    expect(returnVal.missingKey).toBeNull();
  });

  it('allows assigning null to variable type', () => {
    const config = new DynamicConfig(
      'cfg',
      JSON.stringify({ value: { FAKE_BQ_CONFIG: '10' } }),
    );
    const returnVal: BigQueryTablesConfig = {
      FAKE_BQ_CONFIG: config.get('FAKE_BQ_CONFIG', null),
    }
    expect(returnVal.FAKE_BQ_CONFIG).toBe('10');
  });

  it('allows assigning Record<string, string[]> from get()', () => {
    const config = new DynamicConfig(
      'cfg',
      JSON.stringify({ value: { allow_tag_list: { a: ['x'] } } }),
    );

    const allowLists: Record<string, string[]> = config.get('allow_tag_list', {});
    expect(allowLists).toEqual({ a: ['x'] });
  });

  it('allows assigning typed object defaults (FakeSettingsType) from get()', () => {
    const config = new DynamicConfig(
      'cfg',
      JSON.stringify({
        value: {
          fake_settings: {
            async_insert: 1,
            wait_for_async_insert: 1,
            async_insert_max_data_size: '1000000',
            async_insert_busy_timeout_ms: 1000,
          },
        },
      }),
    );

    const fake_settings = config.get(
      'fake_settings',
      {
        async_insert: 1,
        wait_for_async_insert: 1,
        async_insert_max_data_size: '1000000',
        async_insert_busy_timeout_ms: 1000,
      } as FakeSettingsType,
    );

    expect(fake_settings['async_insert']).toBe(1);
  });
});
