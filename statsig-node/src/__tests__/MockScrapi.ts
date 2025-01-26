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

    this.app.use(express.json());

    this.app.use((req: express.Request, res: express.Response) => {
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
    });
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
