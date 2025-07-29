import { BuilderOptions } from '@/commands/builders/builder-options.js';
import { getArchInfo } from '@/utils/docker_utils.js';
import { buildFfiHelper, detectTarget } from '@/utils/ffi_utils.js';
import { BASE_DIR, listFiles } from '@/utils/file_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import fs from 'fs';
import { execSync } from 'node:child_process';
import path from 'node:path';

const JAVA_NATIVE_DIR = path.resolve(
  BASE_DIR,
  'statsig-java/src/main/resources/native',
);

export function buildJava(options: BuilderOptions) {
  Log.title(`Building statsig-java`);

  options.release = true; // default to true
  options.targetProject = 'statsig_java';
  options.target = detectTarget(options);
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

  let fileMoved = false;

  libFiles.forEach((file) => {
    const filename = path.basename(file);
    const destDir = path.resolve(JAVA_NATIVE_DIR, options.target);
    ensureDirExists(destDir);

    const destinationPath = path.join(destDir, filename);
    execSync(`cp ${file} ${destinationPath}`);

    Log.stepProgress(`Copied lib to ${destinationPath}`);
    fileMoved = true;
  });

  if (!fileMoved) {
    Log.stepEnd('No matching native files found to move', 'failure');
  }

  Log.stepEnd('Successfully copied native files');
}

function ensureDirExists(dirPath: string) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
  }
}