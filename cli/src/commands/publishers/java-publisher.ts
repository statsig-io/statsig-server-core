import {
  BASE_DIR,
  ensureEmptyDir,
  getRootedPath,
  listFiles,
} from '@/utils/file_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import { execSync } from 'node:child_process';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

const TARGET_MAPPING = {
  'macos-aarch64-apple-darwin-ffi': 'macos-arm64',
  'debian-aarch64-unknown-linux-gnu-ffi': 'linux-gnu-arm64',
  'amazonlinux2-aarch64-unknown-linux-gnu-ffi': 'amazonlinux2-arm64',
  'amazonlinux2-x86_64-unknown-linux-gnu-ffi': 'amazonlinux2-x86_64',
  'amazonlinux2023-aarch64-unknown-linux-gnu-ffi': 'amazonlinux2023-arm64',
  'amazonlinux2023-x86_64-unknown-linux-gnu-ffi': 'amazonlinux2023-x86_64',
  'centos7-x86_64-unknown-linux-gnu-ffi': 'centos7-x86_64',
  'windows-i686-pc-windows-msvc-ffi': 'windows-i686',
  'macos-x86_64-apple-darwin-ffi': 'macos-x86_64',
  'windows-x86_64-pc-windows-msvc-ffi': 'windows-x86_64',
  'debian-x86_64-unknown-linux-gnu-ffi': 'linux-gnu-x86_64',
};


const JAVA_NATIVE_DIR = path.resolve(
    BASE_DIR,
    'statsig-java/java/src/main/resources/native',
);

export async function javaPublish(options: PublisherOptions) {
  const libFiles = [
    ...listFiles(options.workingDir, '**/target/**/release/*.dylib'),
    ...listFiles(options.workingDir, '**/target/**/release/*.so'),
    ...listFiles(options.workingDir, '**/target/**/release/*.dll'),
  ].filter(isMappedTarget);

  Log.stepBegin('Clearing Java Native Directory');
  ensureEmptyDir(JAVA_NATIVE_DIR);
  Log.stepEnd(`Cleared ${JAVA_NATIVE_DIR}`);

  moveJavaLibraries(libFiles);
  publishJavaPackages(options);
}

function isMappedTarget(file: string): boolean {
  return Object.keys(TARGET_MAPPING).some((target) => file.includes(target));
}

function getDestination(file: string, destKeys: string[]): string | null {
  const found = destKeys.findIndex((key) => file.includes(key));

  if (found !== -1) {
    const value = TARGET_MAPPING[destKeys[found]];
    destKeys.splice(found, 1);
    return value;
  }

  return null;
}

function moveJavaLibraries(libFiles: string[]) {
  Log.stepBegin('Moving Java Libraries');

  const destKeys = Object.keys(TARGET_MAPPING);

  let allFilesMoved = true;
  libFiles.forEach((file) => {
    const destination = getDestination(file, destKeys);
    if (!destination) {
      Log.stepProgress(`No mapping found for: ${file}`, 'failure');
      allFilesMoved = false;
      return;
    }

    const filename = path.basename(file);
    const destDir = path.resolve(JAVA_NATIVE_DIR, destination);
    ensureEmptyDir(destDir);

    const destinationPath = path.resolve(destDir, filename);
    execSync(`cp ${file} ${destinationPath}`);

    Log.stepProgress(`Copied lib to ${destinationPath}`);
  });

  if (!allFilesMoved) {
    Log.stepEnd('Failed to move all files', 'failure');
    throw new Error('Failed to move all files');
  }

  if (destKeys.length > 0) {
    Log.stepEnd(`Unused mappings: \n - ${destKeys.join('\n - ')}`, 'failure');
    throw new Error('Failed to move all files');
  }

  Log.stepEnd('Successfully moved Java Libraries');
}

function publishJavaPackages(options: PublisherOptions) {
  Log.stepBegin('Publishing Java Packages');

  execSync('./gradlew publish', {
    cwd: getRootedPath('statsig-java/java'),
    stdio: 'inherit',
  });

  Log.stepEnd('Successfully published Java Packages');
}
