import { execFileSync } from 'node:child_process';
import http from 'node:http';

import { MockScrapi } from './MockScrapi';

function post(url: string, body: Buffer, encoding?: string): Promise<number> {
  return new Promise((resolve, reject) => {
    const req = http.request(url, {
      method: 'POST',
      agent: false,
      headers: {
        'content-type': 'application/json',
        'content-length': body.length,
        ...(encoding ? { 'content-encoding': encoding } : {}),
      },
    }, (res) => {
      res.resume();
      res.on('end', () => resolve(res.statusCode ?? 0));
      res.on('error', reject);
    });
    req.on('error', reject);
    req.setTimeout(3000, () => req.destroy(new Error('Mock request timed out')));
    req.end(body);
  });
}

describe('MockScrapi request decoding', () => {
  let scrapi: MockScrapi;

  beforeEach(async () => {
    scrapi = await MockScrapi.create();
    scrapi.mock('/v1/log_event', '{"success":true}', { status: 202, method: 'POST' });
  });

  afterEach(() => {
    jest.restoreAllMocks();
    scrapi.close();
  });

  it('records JSON bodies before the response handler', async () => {
    const body = { events: [{ eventName: 'plain-json' }] };
    expect(await post(scrapi.getUrlForPath('/v1/log_event'), Buffer.from(JSON.stringify(body)))).toBe(202);
    expect(scrapi.requests[0].body).toEqual(body);
  });

  it('isolates simultaneous zstd requests with the same timestamp', async () => {
    jest.spyOn(Date, 'now').mockReturnValue(123456789);
    const bodies = Array.from({ length: 8 }, (_, index) => ({
      events: [{ eventName: `request-${index}`, value: 'x'.repeat(128) }],
    }));
    const responses = await Promise.all(bodies.map((body) => {
      const compressed = execFileSync('zstd', ['-q', '-c'], { input: JSON.stringify(body) });
      return post(scrapi.getUrlForPath('/v1/log_event'), compressed, 'zstd');
    }));
    expect(responses).toEqual(bodies.map(() => 202));
    expect(scrapi.requests.map((req) => req.body).sort((a, b) =>
      a.events[0].eventName.localeCompare(b.events[0].eventName),
    )).toEqual(bodies);
  });

  it('rejects invalid zstd without recording an undefined body', async () => {
    jest.spyOn(console, 'error').mockImplementation(() => {});
    expect(await post(scrapi.getUrlForPath('/v1/log_event'), Buffer.from('invalid'), 'zstd')).toBe(400);
    expect(scrapi.requests).toEqual([]);
  });
});
