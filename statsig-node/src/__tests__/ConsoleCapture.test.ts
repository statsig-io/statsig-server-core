import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions } from '../../build/index.js';

import { MockScrapi } from './MockScrapi';
import { startStatsigConsoleCapture } from '../../build/console_capture';

const mockConsole = {
  log: jest.fn(),
  warn: jest.fn(),
  error: jest.fn(),
  debug: jest.fn(),
  info: jest.fn(),
  trace: jest.fn(),
};

const originalConsole = { ...console };
describe('Console Capture', () => {
  let statsig: Statsig;
  let scrapi: MockScrapi;
  let options: StatsigOptions;

  beforeAll(async () => {
    scrapi = await MockScrapi.create();

    const dcs = fs.readFileSync(
      path.join(
        __dirname,
        '../../../statsig-rust/tests/data/eval_proj_dcs.json',
      ),
      'utf8',
    );

    scrapi.mock('/v2/download_config_specs', dcs, {
      status: 200,
      method: 'GET',
    });

    scrapi.mock('/v1/log_event', '{"success": true}', {
      status: 202,
      method: 'POST',
    });

    const specsUrl = scrapi.getUrlForPath('/v2/download_config_specs');
    const logEventUrl = scrapi.getUrlForPath('/v1/log_event');
    options = {
      specsUrl,
      logEventUrl,
    };
  });

  afterAll(async () => {
    await statsig.shutdown();
    scrapi.close();
  });

  beforeEach(() => {
    jest.clearAllMocks();
    Object.assign(console, mockConsole);
    Object.values(mockConsole).forEach(fn => fn.mockClear())
    scrapi.clearRequests();
  });

  afterEach(() => {
    Object.assign(console, originalConsole);
  });

  it('console capture with no active statsig instance noop', () => {
    const sdkKey = 'test-sdk-key';
    startStatsigConsoleCapture(sdkKey);
    
    console.log('test message', { data: 'value' });
    
    expect(mockConsole.log).toHaveBeenCalledWith('test message', { data: 'value' });
  });

  it('console capture log', async () => {
    const sdkKey = 'test-sdk-key';
    statsig = new Statsig(sdkKey, options);

    console.log('test message', { data: 'value' });
    await statsig.initialize();
    await statsig.flushEvents();

    const events = scrapi.getLogEventRequests()[0].body.events;
    expect(events[0]).toMatchObject({
      eventName: 'statsig::log_line',
      value: 'test message {"data":"value"}',
      metadata: {
        log_level: 'Log',
        status: 'info',
        source: 'statsig-server-core-node',
        trace: expect.any(String)
      }
    });
  });

  it('console capture warn', async () => {
    const sdkKey = 'test-sdk-key';
    statsig = new Statsig(sdkKey, options);

    console.warn('test message', { data: 'value' });
    await statsig.initialize();
    await statsig.flushEvents();

    const events = scrapi.getLogEventRequests()[0].body.events;
    expect(events[0]).toMatchObject({
      eventName: 'statsig::log_line',
      value: 'test message {"data":"value"}',
      metadata: {
        log_level: 'Warn',
        status: 'warn',
        source: 'statsig-server-core-node',
        trace: expect.any(String)
      }
    });
  });

  it('console capture trace', async () => {
    // Don't use mock console for this test since we need the real console.trace behavior
    Object.assign(console, originalConsole);
    const sdkKey = 'test-sdk-key';
    statsig = new Statsig(sdkKey, options);
    
    console.trace('test message', { data: 'value' });
    await statsig.initialize();
    await statsig.flushEvents();

    const events = scrapi.getLogEventRequests()[0].body.events;
    expect(events[0]).toMatchObject({
      eventName: 'statsig::log_line',
      value: expect.stringContaining("Trace:"),
      metadata: {
        log_level: 'Trace',
        status: 'trace',
        source: 'statsig-server-core-node',
        trace: expect.any(String)
      }
    });

   // console.trace includes trace stack in the value and will contain the test file name
    expect(events[0].value).toEqual(expect.stringContaining("ConsoleCapture.test.ts"));
  });
}); 