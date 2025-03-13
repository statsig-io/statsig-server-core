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

    const filename = path.basename(file);

    ensureEmptyDir(JAVA_NATIVE_DIR);

    const destinationPath = path.resolve(JAVA_NATIVE_DIR, filename);
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
