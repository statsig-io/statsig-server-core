import { Statsig, StatsigUser } from '../../build';
import { MockScrapi } from './MockScrapi';

describe('EBMessageReplacement', () => {
  let statsig: Statsig;
  let consoleErrorSpy: jest.SpyInstance;
  let scrapi: MockScrapi;

  beforeEach(async () => {
    scrapi = await MockScrapi.create();
    statsig = new Statsig('secret-123', {
      specsUrl: scrapi.getUrlForPath('/v2/download_config_specs'),
      logEventUrl: scrapi.getUrlForPath('/v1/log_event'),
    });
    await statsig.initialize();
    consoleErrorSpy = jest.spyOn(console, 'error');
  });

  afterEach(async () => {
    await statsig.shutdown();
    consoleErrorSpy.mockRestore();
  });

  describe('StatsigUser error message replacement', () => {
    it('should replace the invalid user argument error message for getFeatureGate', async () => {
      // @ts-expect-error - we are testing the error boundary
      statsig.getFeatureGate({ userID: 'a-user' }, 'test_gate');

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        'Statsig::getFeatureGate',
        expect.objectContaining({
          message:
            'Expected StatsigUser instance, plain javascript object is not supported. Please create a StatsigUser instance using `new StatsigUser(...)` instead.',
        })
      );
    });

    it('should replace the invalid user argument error message for getConfig', async () => {
      // @ts-expect-error - we are testing the error boundary
      statsig.getDynamicConfig({ userID: 'a-user' }, 'test_config');

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        'Statsig::getDynamicConfig',
        expect.objectContaining({
          message:
            'Expected StatsigUser instance, plain javascript object is not supported. Please create a StatsigUser instance using `new StatsigUser(...)` instead.',
        })
      );
    });

    it('should replace the invalid user argument error message for getExperiment', async () => {
      // @ts-expect-error - we are testing the error boundary
      statsig.getExperiment({ userID: 'a-user' }, 'test_experiment');

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        'Statsig::getExperiment',
        expect.objectContaining({
          message:
            'Expected StatsigUser instance, plain javascript object is not supported. Please create a StatsigUser instance using `new StatsigUser(...)` instead.',
        })
      );
    });

    it('should replace the invalid user argument error message for getLayer', async () => {
      // @ts-expect-error - we are testing the error boundary
      statsig.getLayer({ userID: 'a-user' }, 'test_layer');

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        'Statsig::getLayer',
        expect.objectContaining({
          message:
            'Expected StatsigUser instance, plain javascript object is not supported. Please create a StatsigUser instance using `new StatsigUser(...)` instead.',
        })
      );
    });

    it('does not replace the error message for other errors', async () => {
      // @ts-expect-error - we are testing the error boundary
      statsig.getFeatureGate(StatsigUser.withUserID('a-user'), null);

      expect(consoleErrorSpy).toHaveBeenCalledWith(
        'Statsig::getFeatureGate',
        expect.not.objectContaining({
          message:
            'Expected StatsigUser instance, plain javascript object is not supported. Please create a StatsigUser instance using `new StatsigUser(...)` instead.',
        })
      );
    });
  });
});
