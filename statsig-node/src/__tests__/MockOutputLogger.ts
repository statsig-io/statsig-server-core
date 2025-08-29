import { OutputLoggerProvider } from '../../build/index.js';

export class MockOutputLogger implements OutputLoggerProvider {
  verbose = false;

  readonly initialize = () => {
    if (this.verbose) {
      console.log('MockOutputLogger: initialize');
    }
  };

  readonly debug = (tag: string, message: string) => {
    if (this.verbose) {
      console.log('MockOutputLogger: debug', tag, message);
    }
  };

  readonly info = (tag: string, message: string) => {
    if (this.verbose) {
      console.log('MockOutputLogger: info', tag, message);
    }
  };

  readonly warn = (tag: string, message: string) => {
    if (this.verbose) {
      console.log('MockOutputLogger: warn', tag, message);
    }
  };

  readonly error = (tag: string, message: string) => {
    if (this.verbose) {
      console.log('MockOutputLogger: error', tag, message);
    }
  };

  readonly shutdown = () => {
    if (this.verbose) {
      console.log('MockOutputLogger: shutdown');
    }
  };
}
