import compression from 'compression';
import express from 'express';
import http from 'http';
import 'jest-extended';
import { execFile } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

type MockOptions = {
  status: number;
  method: 'GET' | 'POST';
  headers?: Record<string, string>;
};

type Mock = {
  response: string | Buffer;
  options: MockOptions | null;
};

type RecordedRequest = {
  path: string;
  method: string;
  body: any;
  url: string;
  headers: Record<string, string>;
};

export class MockScrapi {
  requests: RecordedRequest[] = [];

  private app: express.Application;
  private port: number;
  private server: http.Server;

  private mocks: Record<string, Mock> = {};
  private waiters: ((req: RecordedRequest) => void)[] = [];

  private constructor(onReady: () => void) {
    this.app = express();
    this.port = 0;
    this.server = this.app.listen(0, () => {
      const address = this.server.address();
      if (address && typeof address !== 'string') {
        this.port = address.port;
      }
      onReady();
    });

    const jsonParser = express.json();

    this.app.use(
      (
        req: express.Request,
        _res: express.Response,
        next: express.NextFunction,
      ) => {
        // console.log(`[Scrapi] Req ${req.method}:`, req.url, Date.now());
        next();
      },
      compression({
        filter: (req, res) => {
          if (req.headers['content-encoding'] === 'zstd') {
            return false;
          }
          return compression.filter(req, res);
        },
      }),
      (req: express.Request, res: express.Response, next: express.NextFunction) => {
        // body-parser cannot decode zstd; other JSON requests must be parsed
        // before the recording/response handler below consumes them.
        if (req.headers['content-encoding'] === 'zstd') {
          next();
        } else {
          jsonParser(req, res, next);
        }
      },
      async (req: express.Request, res: express.Response) => {
        if (req.headers['content-encoding'] === 'zstd') {
          if (!(await decompressZstd(req))) {
            res.status(400).send('Invalid zstd request');
            return;
          }
        }

        const recorded = {
          path: req.path,
          method: req.method,
          body: req.body,
          url: req.url,
          headers: Object.fromEntries(Object.entries(req.headers)) as Record<
            string,
            string
          >,
        };

        this.requests.push(recorded);
        this.waiters.forEach((waiter) => waiter(recorded));

        const found = Object.entries(this.mocks).find(([path, mock]) => {
          if (mock.options?.method !== req.method) {
            return false;
          }

          return req.path.startsWith(path);
        });

        if (!found) {
          console.log('Unmatched request:', req.method, req.url);
          res.status(404).send('Not Found');
          return;
        }

        const [_, mock] = found;
        res
          .status(mock.options?.status ?? 200)
          .set(mock.options?.headers ?? {})
          .send(mock.response);
      },
    );
  }

  static async create(): Promise<MockScrapi> {
    return new Promise((resolve) => {
      const scrapi = new MockScrapi(() => resolve(scrapi));
    });
  }

  close() {
    this.server.close();
  }

  getUrlForPath(path: string) {
    return `http://localhost:${this.port}${path}`;
  }

  getLogEventRequests() {
    return this.requests.filter((req: any) => req.path === '/v1/log_event');
  }

  clearRequests() {
    this.requests = [];
  }

  getServerApi() {
    return `http://localhost:${this.port}`;
  }

  mock(path: string, response: string | Buffer, options?: MockOptions) {
    this.mocks[path] = {
      response,
      options: options ?? null,
    };
  }

  waitForNext(filter: (req: RecordedRequest) => boolean) {
    return new Promise<boolean>((resolve) => {
      const myWaiter = (req: RecordedRequest) => {
        if (filter(req)) {
          this.waiters = this.waiters.filter((waiter) => waiter !== myWaiter);

          clearTimeout(timeout);
          resolve(true);
        }
      };

      const timeout = setTimeout(() => {
        this.waiters = this.waiters.filter((waiter) => waiter !== myWaiter);
        resolve(false);
      }, 1000);

      this.waiters.push(myWaiter);
    });
  }
}

export function anyNumericString(): unknown {
  return expect.toSatisfy(
    (input) => typeof input === 'string' && /^[0-9]+$/.test(input),
  );
}

async function decompressZstd(req: express.Request): Promise<boolean> {
  let temporaryDirectory: string | undefined;
  try {
    const chunks: Buffer[] = [];
    for await (const chunk of req) {
      chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
    }

    // Requests in different Jest workers can arrive in the same millisecond.
    // Each decode owns its directory so it cannot overwrite another request.
    temporaryDirectory = await mkdtemp(path.join(os.tmpdir(), 'statsig-zstd-'));
    const inputPath = path.join(temporaryDirectory, 'input');
    const outputPath = path.join(temporaryDirectory, 'output');
    await writeFile(inputPath, Buffer.concat(chunks));
    await promisify(execFile)('zstd', ['-d', inputPath, '-o', outputPath]);
    req.body = JSON.parse(await readFile(outputPath, 'utf8'));
    return true;
  } catch (error) {
    console.error('ZSTD decompression failed:', error);
    return false;
  } finally {
    if (temporaryDirectory) {
      await rm(temporaryDirectory, { recursive: true, force: true }).catch(() => {});
    }
  }
}
