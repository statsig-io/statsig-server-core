
import * as fs from 'fs';
import { Log } from './terminal_utils.js';

export function setOutput(name: string, value: string) {
  const githubOutputPath = process.env.GITHUB_OUTPUT;
  if (!githubOutputPath) {
    Log.info('GITHUB_OUTPUT environment variable is not defined.');
    return;
  }
  // Append output in the format "name=value\n"
  fs.appendFileSync(githubOutputPath, `${name}=${value}\n`);
}