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

/**
 DIR_NAME:
  - FILE_GLOB_PATTERN : DESTINATION_FILE_NAME
 */
const DIR_STRUCTURE = {
  main: {
    'macos-aarch64-apple-darwin-node/**/package.json': 'package.json',
    'macos-aarch64-apple-darwin-node/**/index.d.ts': 'index.d.ts',
    'macos-aarch64-apple-darwin-node/**/index.js': 'index.js',
  },
  'aarch64-apple-darwin': {
    'macos-aarch64-apple-darwin-node/**/aarch64-apple-darwin.package.json':
      'package.json',
    'macos-aarch64-apple-darwin-node/**/statsig-node-core.darwin-arm64.node':
      'statsig-node-core.darwin-arm64.node',
  },
  'aarch64-unknown-linux-gnu': {
    'debian-aarch64-unknown-linux-gnu-node/**/aarch64-unknown-linux-gnu.package.json':
      'package.json',
    'debian-aarch64-unknown-linux-gnu-node/**/statsig-node-core.linux-arm64-gnu.node':
      'statsig-node-core.linux-arm64-gnu.node',
  },
  'aarch64-unknown-linux-musl': {
    'alpine-aarch64-unknown-linux-musl-node/**/aarch64-unknown-linux-musl.package.json':
      'package.json',
    'alpine-aarch64-unknown-linux-musl-node/**/statsig-node-core.linux-arm64-musl.node':
      'statsig-node-core.linux-arm64-musl.node',
  },
  'i686-pc-windows-msvc': {
    'windows-i686-pc-windows-msvc-node/**/i686-pc-windows-msvc.package.json':
      'package.json',
    'windows-i686-pc-windows-msvc-node/**/statsig-node-core.win32-ia32-msvc.node':
      'statsig-node-core.win32-ia32-msvc.node',
  },
  'x86_64-apple-darwin': {
    'macos-x86_64-apple-darwin-node/**/x86_64-apple-darwin.package.json':
      'package.json',
    'macos-x86_64-apple-darwin-node/**/statsig-node-core.darwin-x64.node':
      'statsig-node-core.darwin-x64.node',
  },
  'x86_64-pc-windows-msvc': {
    'windows-x86_64-pc-windows-msvc-node/**/x86_64-pc-windows-msvc.package.json':
      'package.json',
    'windows-x86_64-pc-windows-msvc-node/**/statsig-node-core.win32-x64-msvc.node':
      'statsig-node-core.win32-x64-msvc.node',
  },
  'x86_64-unknown-linux-gnu': {
    'debian-x86_64-unknown-linux-gnu-node/**/x86_64-unknown-linux-gnu.package.json':
      'package.json',
    'debian-x86_64-unknown-linux-gnu-node/**/statsig-node-core.linux-x64-gnu.node':
      'statsig-node-core.linux-x64-gnu.node',
  },
  'x86_64-unknown-linux-musl': {
    'alpine-x86_64-unknown-linux-musl-node/**/x86_64-unknown-linux-musl.package.json':
      'package.json',
    'alpine-x86_64-unknown-linux-musl-node/**/statsig-node-core.linux-x64-musl.node':
      'statsig-node-core.linux-x64-musl.node',
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

  alignNodePackage(options, distDir);
  addOptionalDependenciesToPackageJson(options);
  publishNodePackages(options, distDir);
}

function addOptionalDependenciesToPackageJson(options: PublisherOptions) {
  const distDir = options.workingDir + '/statsig-node/dist';

  Log.stepBegin('Adding optional dependencies to package.json');

  const filepath = path.resolve(distDir, 'main/package.json');

  const binaries = listFiles(distDir, '*.node');

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

  let allPlatformsAligned = true;

  Object.entries(DIR_STRUCTURE).forEach(([platform, files]) => {
    Log.stepBegin(`Aligning ${platform} files`);

    const platformDir = path.resolve(distDir, platform);

    ensureEmptyDir(platformDir);
    Log.stepProgress(`Empty ${platform} directory created`);

    let allFilesMoved = true;
    Object.entries(files).forEach(([source, destination]) => {
      const file = listFiles(options.workingDir, source);
      if (file.length !== 1) {
        allFilesMoved = false;
        Log.stepProgress(
          `Multiple files matched for ${source}, expected 1, found ${file.length}`,
          'failure',
        );
        return;
      }

      const sourcePath = file[0];
      const destinationPath = path.resolve(platformDir, destination);
      execSync(`cp ${sourcePath} ${destinationPath}`);

      Log.stepProgress(`Copied ${source} to ${platform}/${destination}`);
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
      version.isBeta() ? `--tag beta` : '',
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
