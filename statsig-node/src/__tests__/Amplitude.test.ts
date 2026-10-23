import { createServer, Server } from 'http';
import { AddressInfo } from 'net';
import { Amplitude } from '../../build/index.js';

describe('Amplitude', () => {
  let server: Server;
  let serverUrl: string;
  let requests: any[];

  beforeEach(async () => {
    requests = [];
    server = createServer((req, res) => {
      let body = '';
      req.on('data', (chunk) => { body += chunk; });
      req.on('end', () => {
        requests.push({ url: req.url, headers: req.headers, body: JSON.parse(body) });
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end('{"code":200}');
      });
    });
    await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
    serverUrl = `http://127.0.0.1:${(server.address() as AddressInfo).port}/2/httpapi`;
  });

  afterEach(async () => {
    await new Promise<void>((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
  });

  it('initializes and tracks an event through the Rust core', async () => {
    const amplitude = new Amplitude();
    amplitude.init('test-api-key', { serverUrl });
    await amplitude.track({
      eventType: 'Song Played',
      userId: 'user-123',
      eventProperties: { title: 'Hello', repeat: true, tags: ['pop'] },
    });
    expect(requests).toHaveLength(1);
    expect(requests[0].url).toBe('/2/httpapi');
    expect(requests[0].headers['statsig-api-key']).toBeUndefined();
    expect(requests[0].body).toEqual({
      api_key: 'test-api-key',
      events: [{
        event_type: 'Song Played',
        user_id: 'user-123',
        event_properties: { title: 'Hello', repeat: true, tags: ['pop'] },
        time: expect.any(Number),
        insert_id: expect.any(String),
        library: expect.stringMatching(/^amplitude-node-core\//),
      }],
    });
  });

  it('reports missing initialization and invalid events', async () => {
    const amplitude = new Amplitude();
    await expect(amplitude.track({ eventType: 'test', deviceId: 'device-123' })).rejects.toThrow('init');
    expect(() => amplitude.init('')).toThrow('api_key');
    amplitude.init('test-api-key', { serverUrl });
    await expect(amplitude.track({ eventType: 'test' })).rejects.toThrow('user_id or device_id');
    expect(requests).toHaveLength(0);
  });
});
