import { ensureEmptyDir, unzip } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';
import fs from 'node:fs';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

const DIR_STRUCTURE = {
  main: {
    'package.json': '',
    'index.d.ts': '',
    'index.js': '',
  },
  'aarch64-apple-darwin': {
    'aarch64-apple-darwin.package.json': 'package.json',
    'statsig-node-core.darwin-arm64.node': '',
  },
  'aarch64-unknown-linux-gnu': {
    'aarch64-unknown-linux-gnu.package.json': 'package.json',
    'statsig-node-core.linux-arm64-gnu.node': '',
  },
  'aarch64-unknown-linux-musl': {
    'aarch64-unknown-linux-musl.package.json': 'package.json',
    'statsig-node-core.linux-arm64-musl.node': '',
  },
  'i686-pc-windows-msvc': {
    'i686-pc-windows-msvc.package.json': 'package.json',
    'statsig-node-core.win32-ia32-msvc.node': '',
  },
  'x86_64-apple-darwin': {
    'x86_64-apple-darwin.package.json': 'package.json',
    'statsig-node-core.darwin-x64.node': '',
  },
  'x86_64-pc-windows-msvc': {
    'x86_64-pc-windows-msvc.package.json': 'package.json',
    'statsig-node-core.win32-x64-msvc.node': '',
  },
  'x86_64-unknown-linux-gnu': {
    'x86_64-unknown-linux-gnu.package.json': 'package.json',
    'statsig-node-core.linux-x64-gnu.node': '',
  },
  'x86_64-unknown-linux-musl': {
    'x86_64-unknown-linux-musl.package.json': 'package.json',
    'statsig-node-core.linux-x64-musl.node': '',
  },
};

export async function nodePublish(options: PublisherOptions) {
  const buildDir = options.workingDir + '/statsig-node/build';

  let allPlatformsAligned = true;

  Object.entries(DIR_STRUCTURE).forEach(([platform, files]) => {
    Log.stepBegin(`Aligning ${platform} files`);

    const platformDir = path.resolve(
      options.workingDir,
      'statsig-node/dist',
      platform,
    );
    ensureEmptyDir(platformDir);
    Log.stepProgress(`Empty ${platform} directory created`);

    let allFilesMoved = true;
    Object.entries(files).forEach(([source, destination]) => {
      const sourcePath = path.resolve(buildDir, source);
      if (!fs.existsSync(sourcePath)) {
        allFilesMoved = false;
        Log.stepProgress(`Failed to find ${source} in ${buildDir}`, 'failure');
        return;
      }

      const destinationPath = path.resolve(platformDir, destination);
      execSync(`mv ${sourcePath} ${destinationPath}`);

      Log.stepProgress(`Moved ${source} to ${platform}/${destination}`);
    });

    if (!allFilesMoved) {
      allPlatformsAligned = false;
      Log.stepEnd(`Failed to move all files for ${platform}`, 'failure');
      return;
    }

    Log.stepEnd(`Moved all files for ${platform}`);
  });

  if (!allPlatformsAligned) {
    Log.stepEnd('Failed to align all platforms', 'failure');
    return;
  }
}
