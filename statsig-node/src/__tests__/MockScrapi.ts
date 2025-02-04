import { decompress } from '@mongodb-js/zstd';
import compression from 'compression';
import express from 'express';
import http from 'http';

type MockOptions = { status: number; method: 'GET' | 'POST' };

type Mock = {
  response: string;
  options: MockOptions | null;
};

type RecordedRequest = {
  path: string;
  method: string;
  body: any;
};

export class MockScrapi {
  requests: RecordedRequest[] = [];

  private app: express.Application;
  private port: number;
  private server: http.Server;

  private mocks: Record<string, Mock> = {};

  private constructor(onReady: () => void) {
    this.app = express();
    this.port = Math.floor(Math.random() * 2000) + 4000;
    this.server = this.app.listen(this.port, onReady);

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
      async (req: express.Request, res: express.Response) => {
        if (req.headers['content-encoding'] === 'zstd') {
          await decompressZstd(req);
        }

        this.requests.push({
          path: req.path,
          method: req.method,
          body: req.body,
        });

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
        res.status(mock.options?.status ?? 200).send(mock.response);
      },
      express.json(),
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

  mock(path: string, response: string, options?: MockOptions) {
    this.mocks[path] = {
      response,
      options: options ?? null,
    };
  }
}

function decompressZstd(req: express.Request): Promise<boolean> {
  return new Promise((resolve, _reject) => {
    try {
      const chunks: Buffer[] = [];
      req.on('data', (chunk: Buffer) => chunks.push(chunk));
      req.on('end', async () => {
        const buffer = Buffer.concat(chunks);
        const decompressed = await decompress(buffer);
        req.body = JSON.parse(decompressed.toString());

        resolve(true);
      });
    } catch (error) {
      resolve(false);
    }
  });
}
