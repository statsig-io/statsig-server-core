import { DynamicConfig } from '../lib/statsig_types';

interface SettingsMap {
  [key: string]: string | number | boolean | SettingsMap | undefined;
}

// fake use cases copied from our services
type FakeSettingsType = Record<
  string,
  string | number | boolean | SettingsMap | undefined
>;

describe('DynamicConfig.get typing regressions', () => {
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
