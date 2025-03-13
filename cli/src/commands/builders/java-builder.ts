import { BuilderOptions } from '@/commands/builders/builder-options.js';
import { getArchInfo } from '@/utils/docker_utils.js';
import { buildFfiHelper } from '@/utils/ffi_utils.js';
import { BASE_DIR, ensureEmptyDir, listFiles } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'node:child_process';
import path from 'node:path';

const JAVA_NATIVE_DIR = path.resolve(
  BASE_DIR,
  'statsig-ffi/bindings/java/src/main/resources/native',
);

const TARGET_MAPPING = {
  'aarch64-macos': 'macos-arm64',
  'aarch64-debian': 'linux-gnu-arm64',
  'x86_64-debian': 'linux-gnu-x86_64',
};

export function buildJava(options: BuilderOptions) {
  Log.title(`Building statsig-java`);

  options.release = true; // default to true
  buildFfiHelper(options);

  Log.stepEnd(`Built statsig-java`);

  const libFiles = [
    ...listFiles(BASE_DIR, '**/target/**/release/*.dylib'),
    ...listFiles(BASE_DIR, '**/target/**/release/*.so'),
    ...listFiles(BASE_DIR, '**/target/**/release/*.dll'),
  ];

  moveJavaLibraries(libFiles, options);
}

function moveJavaLibraries(libFiles: string[], options: BuilderOptions) {
  Log.stepBegin('Moving Java Libraries');

  const { name } = getArchInfo(options.arch);
  const tag = `${name}-${options.os}`;

  let fileMoved = false;
  libFiles.forEach((file) => {
    if (!file.includes(tag)) {
      return;
    }

    const destination = TARGET_MAPPING[tag];
    if (!destination) {
      Log.stepProgress(`No mapping found for: ${file}`, 'failure');
      return;
    }

    const filename = path.basename(file);
    const destDir = path.resolve(JAVA_NATIVE_DIR, destination);
    ensureEmptyDir(destDir);

    const destinationPath = path.resolve(destDir, filename);
    execSync(`cp ${file} ${destinationPath}`);

    Log.stepProgress(`Copied lib to ${destinationPath}`);

    fileMoved = true;
  });

  if (!fileMoved) {
    Log.stepEnd(`Failed to copy native file for tag ${tag}`, 'failure');
    throw new Error(`Failed to copy native file for tag ${tag}`);
  }

  Log.stepEnd(`Successfully copied native file for tag ${tag}`);
}
