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

const TARGETS = [
  'aarch64-apple-darwin',
  'x86_64-apple-darwin',
  'aarch64-unknown-linux-gnu',
  'x86_64-unknown-linux-gnu',
  'x86_64-unknown-linux-musl',
  'aarch64-unknown-linux-musl',
  'x86_64-pc-windows-msvc',
  'i686-pc-windows-msvc',
];

const JAVA_NATIVE_DIR = path.resolve(
  BASE_DIR,
  'statsig-java/src/main/resources/native',
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
  return TARGETS.some((target) => file.includes(target));
}

function getDestination(file: string, destKeys: string[]): string | null {
  const found = destKeys.findIndex((key) => file.includes(key));

  if (found !== -1) {
    const value = destKeys[found];
    return value;
  }

  return null;
}

function moveJavaLibraries(libFiles: string[]) {
  Log.stepBegin('Moving Java Libraries');

  let allFilesMoved = true;
  let movedFiles = 0;
  libFiles.forEach((file) => {
    const destination = getDestination(file, TARGETS);
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
    ++movedFiles;
    Log.stepProgress(`Copied lib to ${destinationPath}`);
  });

  if (!allFilesMoved) {
    Log.stepEnd('Failed to move all files', 'failure');
    throw new Error('Failed to move all files');
  }

  if (movedFiles < TARGETS.length) {
    Log.stepEnd(
      `Moved only ${movedFiles} of ${TARGETS.length} expected files`,
      'failure',
    );
    throw new Error('Failed to move all files');
  }

  Log.stepEnd('Successfully moved Java Libraries');
}

function publishJavaPackages(options: PublisherOptions) {
  Log.stepBegin('Publishing Java Packages');

  execSync(
    './gradlew publishToSonatype closeAndReleaseSonatypeStagingRepository',
    {
      cwd: getRootedPath('statsig-java'),
      stdio: 'inherit',
    },
  );

  Log.stepEnd('Successfully published Java Packages');
}
