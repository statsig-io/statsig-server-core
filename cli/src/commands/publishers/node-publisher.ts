import { unzip } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';
import fs from 'node:fs';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

const DIR_STRUCTURE = {
  main: {
    'package.json': '',
  },
  'aarch64-apple-darwin': {
    'package.json': '',
    'statsig-node-core.darwin-x64.node': '',
  },
  'aarch64-unknown-linux-gnu': {
    'package.json': '',
    'statsig-node-core.linux-arm64-gnu.node': '',
  },
  'aarch64-unknown-linux-musl': {
    'package.json': '',
    'statsig-node-core.linux-arm64-musl.node': '',
  },
  'i686-pc-windows-msvc': {
    'package.json': '',
    'statsig-node-core.win32-ia32-msvc.node': '',
  },
  'x86_64-apple-darwin': {
    'package.json': '',
    'statsig-node-core.darwin-x64.node': '',
  },
  'x86_64-pc-windows-msvc': {
    'package.json': '',
    'statsig-node-core.win32-x64-msvc.node': '',
  },
  'x86_64-unknown-linux-gnu': {
    'package.json': '',
    'statsig-node-core.linux-x64-gnu.node': '',
  },
  'x86_64-unknown-linux-musl': {
    'package.json': '',
    'statsig-node-core.linux-x64-musl.node': '',
  },
};

export async function nodePublish(options: PublisherOptions) {
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
