import { createServer, Server } from 'http';
import { AddressInfo } from 'net';
import { execFile } from 'child_process';
import { promisify } from 'util';
import { Amplitude, AmplitudeEvent } from '../../build/index.js';

const turn = () => new Promise<void>((resolve) => setImmediate(resolve));

describe('Amplitude Java compatibility through the Rust Node binding', () => {
  let server: Server;
  let serverUrl: string;
  let requests: any[];
  let clients: Amplitude[];
  let respond: (body: any, index: number) => { code: number; body: any };

  function client(): Amplitude {
    const result = new Amplitude();
    result.init('test-api-key', { serverUrl });
    result.setLogMode('OFF');
    clients.push(result);
    return result;
  }
  function event(eventType = 'test'): AmplitudeEvent { return { eventType, userId: 'user-123' }; }
  async function flush(amplitude: Amplitude) { await amplitude.flushEvents(); await turn(); }

  beforeEach(async () => {
    requests = [];
    clients = [];
    respond = () => ({ code: 200, body: { code: 200 } });
    server = createServer((req, res) => {
      let body = '';
      req.on('data', (chunk) => { body += chunk; });
      req.on('end', () => {
        const parsed = JSON.parse(body);
        requests.push({ url: req.url, headers: req.headers, body: parsed });
        const response = respond(parsed, requests.length - 1);
        res.writeHead(response.code, { 'Content-Type': 'application/json', Connection: 'close' });
        res.end(JSON.stringify(response.body));
      });
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    serverUrl = `http://127.0.0.1:${(server.address() as AddressInfo).port}/2/httpapi`;
  });

  afterEach(async () => {
    await Promise.all(clients.map((client) => client.shutdown()));
    await new Promise<void>((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
  });

  it('retains immediate track and serializes the complete Java event', async () => {
    const amplitude = client();
    await amplitude.track({
      eventType: 'Song Played', userId: 'user-123',
      eventProperties: { title: 'Hello', repeat: true, tags: ['pop'] },
      timestamp: 12345, locationLat: 1.2, locationLng: -3.4,
      appVersion: '1.2', deviceBrand: 'brand', androidId: 'android', partnerId: 'partner',
      price: 2.5, quantity: 0, revenue: 10, productId: 'sku', revenueType: 'purchase', currency: 'USD',
      eventId: 7, groups: { org: 'amplitude' }, groupProperties: { size: 10 },
    });
    expect(requests).toHaveLength(1);
    expect(requests[0].headers['statsig-api-key']).toBeUndefined();
    expect(requests[0].body.api_key).toBe('test-api-key');
    expect(requests[0].body.events[0]).toMatchObject({
      event_type: 'Song Played', user_id: 'user-123', device_id: null,
      event_properties: { title: 'Hello', repeat: true, tags: ['pop'] },
      time: 12345, location_lat: 1.2, location_lng: -3.4, app_version: '1.2',
      device_brand: 'brand', android_id: 'android', partner_id: 'partner',
      price: 2.5, quantity: 1, revenue: 10, productId: 'sku', revenueType: 'purchase', currency: 'USD',
      event_id: 7, session_id: -1, groups: { org: 'amplitude' }, group_properties: { size: 10 },
      insert_id: expect.any(String), library: expect.stringMatching(/^amplitude-node-core\//),
    });
  });

  it('reports missing initialization and invalid events', async () => {
    const amplitude = new Amplitude(); clients.push(amplitude);
    await expect(amplitude.track({ eventType: 'test', deviceId: 'device-123' })).rejects.toThrow('init');
    expect(() => amplitude.init('')).toThrow('api_key');
    amplitude.init('test-api-key', { serverUrl });
    await expect(amplitude.track({ eventType: 'test' })).rejects.toThrow('user_id or device_id');
    expect(requests).toHaveLength(0);
  });

  it('shares named instances and replaces them after shutdown', async () => {
    const a = Amplitude.getInstance(); const b = Amplitude.getInstance('');
    expect(a).toBe(b);
    const named = Amplitude.getInstance('node-parity'); expect(named).not.toBe(a);
    clients.push(a, named);
    await a.shutdown();
    const replacement = Amplitude.getInstance(null); clients.push(replacement); expect(replacement).not.toBe(a);
  });

  it('buffers ten events by default and supports explicit flush', async () => {
    const amplitude = client();
    for (let index = 0; index < 9; index++) await amplitude.logEvent(event());
    expect(requests).toHaveLength(0);
    await amplitude.logEvent(event()); await flush(amplitude);
    expect(requests).toHaveLength(1); expect(requests[0].body.events).toHaveLength(10);
    await amplitude.logEvent(event('manual')); await flush(amplitude);
    expect(requests).toHaveLength(2); expect(requests[1].body.events).toHaveLength(1);
  });

  it('supports metadata, options, custom headers, callbacks, and clearing callbacks', async () => {
    const amplitude = client(); const clientCallback = jest.fn(); const eventCallback = jest.fn();
    expect(amplitude.setEventUploadThreshold(2)).toBe(amplitude);
    amplitude.setPlan({ source: 'default', versionId: 'v1' });
    amplitude.setIngestionMetadata({ sourceName: 'ampli', sourceVersion: '2' });
    amplitude.setOptions({ minIdLength: 1, headers: { 'X-Custom': 'value' } });
    amplitude.setCallbacks({ onLogEventServerResponse: clientCallback });
    await amplitude.logEvent(event('one'), eventCallback);
    await amplitude.logEvent({ ...event('two'), plan: { source: 'event' } }); await flush(amplitude);
    expect(clientCallback).toHaveBeenCalledTimes(2); expect(eventCallback).toHaveBeenCalledTimes(1);
    expect(eventCallback.mock.calls[0][1]).toBe(200);
    expect(requests[0].body.options).toEqual({ min_id_length: 1 }); expect(requests[0].headers['x-custom']).toBe('value');
    expect(requests[0].body.events[0].plan).toEqual({ source: 'default', versionId: 'v1' });
    expect(requests[0].body.events[1].plan).toEqual({ source: 'event' });
    expect(requests[0].body.events[0].ingestion_metadata).toEqual({ source_name: 'ampli', source_version: '2' });
    amplitude.setCallbacks(null); await amplitude.logEvent(event('cleared')); await flush(amplitude);
    expect(clientCallback).toHaveBeenCalledTimes(2);
  });

  it('runs continuation middleware in order, forwards extra, and drops without next', async () => {
    const amplitude = client(); const order: string[] = [];
    amplitude.addEventMiddleware(async (payload, next) => {
      order.push('before-a'); payload.event.eventProperties = { extra: payload.extra };
      await next(payload); order.push('after-a');
    });
    amplitude.addEventMiddleware(async (payload, next) => {
      order.push('before-b'); payload.event.eventType = 'changed'; await next(payload); order.push('after-b');
    });
    await amplitude.logEvent(event(), null, { request: 1 }); await flush(amplitude);
    expect(order).toEqual(['before-a', 'before-b', 'after-b', 'after-a']);
    expect(requests[0].body.events[0].event_type).toBe('changed');
    expect(requests[0].body.events[0].event_properties).toEqual({ extra: { request: 1 } });
    amplitude.addEventMiddleware(() => {});
    await amplitude.logEvent(event('drop')); await flush(amplitude); expect(requests).toHaveLength(1);
  });

  it('rejects middleware exceptions without sending the event', async () => {
    const amplitude = client(); amplitude.addEventMiddleware(() => { throw new Error('middleware failed'); });
    await expect(amplitude.logEvent(event())).rejects.toThrow('middleware failed'); expect(requests).toHaveLength(0);
  });

  it('filters invalid indices and retries only remaining events', async () => {
    respond = (_, index) => index === 0
      ? { code: 400, body: { code: 400, error: 'bad field', events_with_invalid_fields: { event_type: [1] } } }
      : { code: 200, body: { code: 200 } };
    const amplitude = client(); const callback = jest.fn(); amplitude.setCallbacks(callback);
    for (const name of ['one', 'bad', 'three']) await amplitude.logEvent(event(name));
    await flush(amplitude);
    expect(requests).toHaveLength(2); expect(requests[1].body.events.map((e: any) => e.event_type)).toEqual(['one', 'three']);
    expect(callback).toHaveBeenCalledTimes(3); expect(callback.mock.calls.find(([e]) => e.eventType === 'bad')[1]).toBe(400);
    expect(requests[0].body.events[0].insert_id).toBe(requests[1].body.events[0].insert_id);
  });

  it('honors the custom URL in batch mode and ignores invalid server URLs', async () => {
    const amplitude = client(); amplitude.useBatchMode(true).setServerUrl('invalid').setSendThreadPool(2).setRetryThreadPool(1);
    await amplitude.track(event()); expect(requests[0].url).toBe('/2/httpapi');
    expect(() => amplitude.setSendThreadPool(0)).toThrow('positive');
  });

  it('truncates nested properties and exposes a custom logger', async () => {
    const amplitude = client(); const debug = jest.fn(); const callback = jest.fn();
    amplitude.setLogger({ debug }).setLogMode('DEBUG').setCallbacks(callback);
    await amplitude.track({ ...event(), eventProperties: { nested: ['x'.repeat(1001)] } }); await turn();
    expect(requests[0].body.events[0].event_properties.nested[0]).toHaveLength(1000); expect(debug).toHaveBeenCalled();
    expect(callback.mock.calls[0][0].eventProperties.nested[0]).toHaveLength(1000);
  });

  it('uses the configured proxy instead of connecting to the destination', async () => {
    const amplitude = client();
    amplitude.setProxy({ proxyHost: '127.0.0.1', proxyPort: (server.address() as AddressInfo).port, proxyProtocol: 'http' });
    amplitude.setServerUrl('http://amplitude.invalid/2/httpapi');
    await amplitude.track(event('proxy'));
    expect(requests).toHaveLength(1);
    expect(requests[0].url).toBe('http://amplitude.invalid/2/httpapi');
  });

  it('ignores environment proxy settings by default like Java Proxy.NO_PROXY', async () => {
    const script = `
      const { Amplitude } = require(process.argv[1]);
      (async () => {
        const client = new Amplitude();
        client.init('test-api-key', { serverUrl: process.argv[2] });
        await client.track({ eventType: 'direct', userId: 'user-123' });
        await client.shutdown();
      })().catch(error => { console.error(error); process.exitCode = 1; });
    `;
    const proxy = serverUrl.replace('/2/httpapi', '');
    await promisify(execFile)(process.execPath, ['-e', script, require.resolve('../../build/index.js'), serverUrl], {
      env: { ...process.env, HTTP_PROXY: proxy, http_proxy: proxy, HTTPS_PROXY: proxy, https_proxy: proxy,
        ALL_PROXY: proxy, all_proxy: proxy, NO_PROXY: '', no_proxy: '' },
    });
    expect(requests).toHaveLength(1);
    expect(requests[0].url).toBe('/2/httpapi');
  });

  it('reports buffered events on shutdown without uploading and rejects further events', async () => {
    const amplitude = client(); const callback = jest.fn(); amplitude.setCallbacks(callback);
    await amplitude.logEvent(event('pending')); await amplitude.shutdown(); await turn();
    expect(callback).toHaveBeenCalledWith(expect.objectContaining({ eventType: 'pending' }), 0, 'Client shutdown. Events not retry.');
    expect(requests).toHaveLength(0); await expect(amplitude.logEvent(event('late'))).rejects.toThrow('shut down');
  });
});
