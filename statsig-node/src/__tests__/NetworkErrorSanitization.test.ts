import { Statsig, StatsigOptions } from '../../build/index.js';

describe('Network Error Sanitization', () => {
  let consoleErrorSpy: jest.SpyInstance;

  beforeEach(() => {
    consoleErrorSpy = jest.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  it('masks secret key when logged via console.error', async () => {
    const sdkKey = 'secret-fakeO1234567890';
    const options: StatsigOptions = {
      // Unreachable host to force a network error during initialize.
      specsUrl: 'http://127.0.0.1:1/v2/download_config_specs',
      initTimeoutMs: 1000,
      outputLogLevel: 'none',
    };

    const statsig = new Statsig(sdkKey, options);
    const result = await statsig.initialize();

    expect(result.isSuccess).toBe(false);
    const error = result.error ?? '';

    console.error(error);

    expect(error).toContain('secret-fakeO*****');
    expect(error).not.toContain('secret-fakeO1234567890');
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      expect.stringContaining('secret-fakeO*****'),
    );

    await statsig.shutdown();
  });
});
