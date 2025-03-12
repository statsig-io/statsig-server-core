import {
  ensureEmptyDir,
  getRootedPath,
  listFiles,
  unzip,
} from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
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

const PACKAGE_MAPPING = {
  'darwin-arm64': '@statsig/statsig-node-core-darwin-arm64',
  'linux-arm64-gnu': '@statsig/statsig-node-core-linux-arm64-gnu',
  'linux-arm64-musl': '@statsig/statsig-node-core-linux-arm64-musl',
  'win32-ia32-msvc': '@statsig/statsig-node-core-win32-ia32-msvc',
  'darwin-x64': '@statsig/statsig-node-core-darwin-x64',
  'win32-x64-msvc': '@statsig/statsig-node-core-win32-x64-msvc',
  'linux-x64-gnu': '@statsig/statsig-node-core-linux-x64-gnu',
  'linux-x64-musl': '@statsig/statsig-node-core-linux-x64-musl',
};

export async function nodePublish(options: PublisherOptions) {
  const distDir = path.resolve(options.workingDir, 'statsig-node/dist');

  addOptionalDependenciesToPackageJson(options);
  alignNodePackage(options, distDir);
  publishNodePackages(options, distDir);
}

function addOptionalDependenciesToPackageJson(options: PublisherOptions) {
  const buildDir = options.workingDir + '/statsig-node/build';

  Log.stepBegin('Adding optional dependencies to package.json');

  const filepath = path.resolve(buildDir, 'package.json');

  const binaries = listFiles(buildDir, '*.node');

  const contents = fs.readFileSync(filepath, 'utf8');
  const json = JSON.parse(contents);
  const version = json['version'];

  const optionalDependencies = {};
  binaries.forEach((binary) => {
    const platform = path
      .basename(binary)
      .match(/statsig-node-core\.(.*)\.node/)?.[1];

    const optPackage = PACKAGE_MAPPING[platform];
    if (!optPackage) {
      Log.stepProgress(`Failed to find mapping for ${platform}`, 'failure');
    }

    optionalDependencies[optPackage] = version;

    Log.stepProgress(`${platform} mapped to ${optPackage}`);
  });

  json['optionalDependencies'] = optionalDependencies;

  fs.writeFileSync(filepath, JSON.stringify(json, null, 2), 'utf8');

  console.log(JSON.stringify(optionalDependencies, null, 2));

  Log.stepEnd('Updated all package.json files');
}

function alignNodePackage(options: PublisherOptions, distDir: string) {
  Log.title('Aligning Node Packages');

  const buildDir = options.workingDir + '/statsig-node/build';

  let allPlatformsAligned = true;

  Object.entries(DIR_STRUCTURE).forEach(([platform, files]) => {
    Log.stepBegin(`Aligning ${platform} files`);

    const platformDir = path.resolve(distDir, platform);

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
    process.exit(1);
  }
}

function publishNodePackages(options: PublisherOptions, distDir: string) {
  Log.title('Publishing Node Packages');

  let allPackagesPublished = true;
  Object.keys(DIR_STRUCTURE).forEach((platform) => {
    const platformDir = path.resolve(distDir, platform);

    Log.stepBegin(`Publishing ${platform} package`);

    const version = getRootVersion();

    const configPath = getRootedPath('.npmrc');
    const publish = [
      `npm publish`,
      `--registry=https://registry.npmjs.org/`,
      `--userconfig=${configPath}`,
      `--access public`,
      version.isBeta ? `--tag beta` : '',
    ];

    const command = publish.join(' ');
    try {
      execSync(command, { cwd: platformDir });
      return null;
    } catch (error) {
      allPackagesPublished = false;
      Log.stepProgress(`Failed to publish ${platform}`, 'failure');
    }
  });

  if (!allPackagesPublished) {
    Log.stepEnd('Failed to publish all packages', 'failure');
    process.exit(1);
  }

  Log.stepEnd('Published all packages', 'success');
}
