import {
  getArchInfo,
  getDockerImageTag,
  isLinux,
} from '@/utils/docker_utils.js';
import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import { unzip } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';
import fs from 'node:fs';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

export async function publishPython(options: PublisherOptions) {
  const files = listFiles(options.workingDir, '*.zip');
  console.log(files);

  const extractTo = unzipFiles(files, options);
  const unzipped = listFiles(extractTo, 'statsig_python_core*');

  console.log(unzipped);

  Log.stepBegin('Uploading to PyPI');

  unzipped.forEach((file) => {
    Log.stepBegin(`\nUploading ${path.basename(file)}`);
    const command = [
      'maturin upload',
      '--non-interactive',
      '--skip-existing',
      '--verbose',
      file,
    ].join(' ');

    Log.stepEnd(command);

    execSync(command, { cwd: options.workingDir, stdio: 'inherit' });
  });
}

function listFiles(dir: string, pattern: string) {
  return execSync(`find ${dir} -name "${pattern}"`)
    .toString()
    .trim()
    .split('\n');
}

function unzipFiles(files: string[], options: PublisherOptions) {
  const extractTo = path.resolve(options.workingDir, 'unzipped');

  files.forEach((file) => {
    const filepath = path.resolve(file);
    const name = path.basename(filepath).replace('.zip', '');

    const buffer = fs.readFileSync(filepath);
    unzip(buffer, extractTo);
    Log.stepProgress(`Completed: ${name}`);
  });

  return extractTo;
}
