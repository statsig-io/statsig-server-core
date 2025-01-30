import { unzip } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';
import fs from 'node:fs';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

export async function ffiPublish(options: PublisherOptions) {
  const files = listFiles(options.workingDir, '*.zip');
  const extractTo = unzipFiles(files, options);
  const binaries = listFiles(extractTo, '*.node');
  Log.stepBegin('Uploading to NPM');
  binaries.forEach((file) => {
    Log.stepBegin(`\nUploading ${path.basename(file)}`);
  });
}

function listFiles(dir: string, pattern: string) {
  return execSync(`find ${dir} -name "${pattern}"`)
    .toString()
    .trim()
    .split('\n');
}

function unzipFiles(files: string[], options: PublisherOptions) {
  Log.stepBegin('Unzipping files');

  const extractTo = path.resolve(options.workingDir, 'unzipped');

  files.forEach((file) => {
    const filepath = path.resolve(file);
    const name = path.basename(filepath).replace('.zip', '');

    const buffer = fs.readFileSync(filepath);
    unzip(buffer, extractTo);
    Log.stepProgress(`Completed: ${name}`);
  });

  Log.stepEnd('Unzipped all files');

  return extractTo;
}
