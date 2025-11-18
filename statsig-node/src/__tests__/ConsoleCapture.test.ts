import * as fs from 'node:fs';
import * as path from 'node:path';

import { Statsig, StatsigOptions, StatsigUser } from '../../build/index.js';
import {
  startStatsigConsoleCapture,
  stopStatsigConsoleCapture,
} from '../../build/console_capture';

import { MockScrapi } from './MockScrapi';

const mockConsole = {
  log: jest.fn(),
  warn: jest.fn(),
  error: jest.fn(),
  debug: jest.fn(),
  info: jest.fn(),
  trace: jest.fn(),
};

const originalConsole = { ...console };

// need to use different sdk key for each test to get around global console capture registry
describe('Console Capture', () => {
  let statsig: Statsig | undefined;
  let scrapi: MockScrapi;
  let options: StatsigOptions;

  beforeAll(async () => {
    scrapi = await MockScrapi.create();

    const dcs = fs.readFileSync(
      path.join(
        __dirname,
        '../../../statsig-rust/tests/data/eval_proj_dcs.json'
      ),
      'utf8'
    );

    scrapi.mock('/v2/download_config_specs', dcs, {
      status: 200,
      method: 'GET',
    });

    scrapi.mock('/v1/log_event', '{"success": true}', {
      status: 202,
      method: 'POST',
    });

    options = {
      specsUrl: scrapi.getUrlForPath('/v2/download_config_specs'),
      logEventUrl: scrapi.getUrlForPath('/v1/log_event'),
    };
  });

  afterAll(async () => {
    scrapi.close();
  });

  beforeEach(() => {
    jest.clearAllMocks();
    scrapi.clearRequests();
  });

  afterEach(async () => {
    if (statsig) {
      await statsig.shutdown();
    }
    statsig = undefined;
  });

  it('console capture with no active statsig instance noop', () => {
    const mockConsole = { ...console, log: jest.fn() };
    Object.assign(console, mockConsole);

    const sdkKey = 'test-sdk-key-1';
    statsig = new Statsig(sdkKey, { consoleCaptureOptions: { enabled: true } });
    // drop statsig instance
    statsig = undefined;

    console.log('test message', { data: 'value' });
    expect(mockConsole.log).toHaveBeenCalledWith('test message', {
      data: 'value',
    });

    Object.assign(console, globalThis.console);
  });

  it('console capture log default log levels', async () => {
    const sdkKey = 'test-sdk-key';
    options.consoleCaptureOptions = {
      enabled: true,
    };
    statsig = new Statsig(sdkKey, options);

    console.log('test message', { data: 'info' });
    console.warn('test message', { data: 'warn' });
    console.error('test message', { data: 'error' });
    await statsig.initialize();
    await statsig.flushEvents();

    const events = scrapi.getLogEventRequests()[0].body.events;
    const logEvents = events.filter(
      (event: any) => event.eventName === 'statsig::log_line'
    );

    expect(logEvents).toHaveLength(2);
    expect(logEvents[0]).toMatchObject({
      eventName: 'statsig::log_line',
      value: 'test message {"data":"warn"}',
      metadata: {
        log_level: 'Warn',
        status: 'warn',
        source: 'statsig-server-core-node',
        trace: expect.any(String),
      },
    });
    expect(logEvents[1]).toMatchObject({
      eventName: 'statsig::log_line',
      value: 'test message {"data":"error"}',
      metadata: {
        log_level: 'Error',
        status: 'error',
        source: 'statsig-server-core-node',
        trace: expect.any(String),
      },
    });
  });

  it('console capture allowed log levels (trace)', async () => {
    const sdkKey = 'test-sdk-key-2';
    options.consoleCaptureOptions = {
      enabled: true,
      logLevels: ['trace' as const],
    };
    statsig = new Statsig(sdkKey, options);

    console.trace('test message', { data: 'value' });
    await statsig.initialize();
    await statsig.flushEvents();

    const events = scrapi.getLogEventRequests()[0].body.events;
    const logEvents = events.filter(
      (event: any) => event.eventName === 'statsig::log_line'
    );

    expect(logEvents[0]).toMatchObject({
      eventName: 'statsig::log_line',
      value: expect.stringContaining('Trace:'),
      metadata: {
        log_level: 'Trace',
        status: 'trace',
        source: 'statsig-server-core-node',
        trace: expect.any(String),
      },
    });

    // console.trace includes trace stack in the value and will contain the test file name
    expect(events[0].value).toEqual(
      expect.stringContaining('ConsoleCapture.test.ts')
    );
  });

  it('console capture with user', async () => {
    const sdkKey = 'test-sdk-key-3';
    const user = StatsigUser.withUserID('test-user-id');
    options.consoleCaptureOptions = {
      enabled: true,
      user: user,
      logLevels: ['log' as const],
    };
    statsig = new Statsig(sdkKey, options);

    console.log('test message', { data: 'value' });
    await statsig.initialize();
    await statsig.flushEvents();

    const events = scrapi.getLogEventRequests()[0].body.events;
    const logEvents = events.filter(
      (event: any) => event.eventName === 'statsig::log_line'
    );
    expect(logEvents).toHaveLength(1);
    expect(logEvents[0]).toMatchObject({
      eventName: 'statsig::log_line',
      value: 'test message {"data":"value"}',
      metadata: {
        log_level: 'Log',
        status: 'info',
        source: 'statsig-server-core-node',
        trace: expect.any(String),
      },
    });
    expect(logEvents[0].user).toMatchObject({
      userID: 'test-user-id',
    });
  });

  it('does not capture log when not enabled', async () => {
    const sdkKey = 'test-sdk-key-4';
    options.consoleCaptureOptions = undefined;
    statsig = new Statsig(sdkKey, options);

    console.log('test message', { data: 'value' });
    await statsig.initialize();
    await statsig.flushEvents();

    const events = scrapi.getLogEventRequests()?.[0]?.body?.events ?? [];
    const logEvents = events.filter(
      (event: any) => event.eventName === 'statsig::log_line'
    );
    expect(logEvents).toHaveLength(0);
  });

  it('stops capturing log when statsig is shutdown', async () => {
    const sdkKey = 'test-sdk-key-5';
    options.consoleCaptureOptions = {
      enabled: true,
      logLevels: ['log', 'warn', 'error'] as const,
    };
    options.eventLoggingFlushIntervalMs = 10;
    statsig = new Statsig(sdkKey, options);

    console.log('test message', { data: 'value' });
    await statsig.initialize();
    await statsig.flushEvents();

    let events = scrapi.getLogEventRequests()[0].body.events;
    let logEvents = events.filter(
      (event: any) => event.eventName === 'statsig::log_line'
    );
    expect(logEvents).toHaveLength(1);
    await statsig.shutdown();

    console.log('test message', { data: 'value' });
    console.warn('test message', { data: 'value' });
    console.error('test message', { data: 'value' });

    await new Promise((resolve) => setTimeout(resolve, 500));

    events = scrapi.getLogEventRequests()[0].body.events;
    logEvents = events.filter(
      (event: any) => event.eventName === 'statsig::log_line'
    );
    expect(logEvents).toHaveLength(1);
  });

  it('does not capture internal logs', async () => {
    const warnSpy = jest.spyOn(console, 'warn');
    const sdkKey = 'test-sdk-key-6';
    options.consoleCaptureOptions = {
      enabled: true,
      logLevels: ['log', 'warn', 'error'] as const,
    };
    statsig = new Statsig(sdkKey, options);
    await statsig.initialize();

    Statsig.shared().checkGate(
      StatsigUser.withUserID('test-user-id'),
      'test_public'
    );

    await new Promise((resolve) => setTimeout(resolve, 500)); // extra buffer to allow the log to be captured if any
    await statsig.flushEvents();

    expect(warnSpy).toHaveBeenCalledWith(
      '[Statsig] No shared instance has been created yet. Call newShared() before using it. Returning an invalid instance'
    );

    const events = scrapi.getLogEventRequests()[0].body.events;
    const logEvents = events.filter(
      (event: any) => event.eventName === 'statsig::log_line'
    );
    expect(logEvents).toHaveLength(0);
  });
});
