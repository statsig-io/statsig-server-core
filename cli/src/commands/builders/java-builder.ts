import { BuilderOptions } from '@/commands/builders/builder-options.js';
import { getArchInfo } from '@/utils/docker_utils.js';
import { buildFfiHelper } from '@/utils/ffi_utils.js';
import { BASE_DIR, listFiles } from '@/utils/file_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import fs from 'fs';
import { execSync } from 'node:child_process';
import path from 'node:path';

const TARGET_MAPPING = {
  'macos-aarch64-apple-darwin-java': 'macos-arm64',
  'debian-aarch64-unknown-linux-gnu-java': 'linux-gnu-arm64',
  'amazonlinux2-aarch64-unknown-linux-gnu-java': 'amazonlinux2-arm64',
  'amazonlinux2-x86_64-unknown-linux-gnu-java': 'amazonlinux2-x86_64',
  'amazonlinux2023-aarch64-unknown-linux-gnu-java': 'amazonlinux2023-arm64',
  'amazonlinux2023-x86_64-unknown-linux-gnu-java': 'amazonlinux2023-x86_64',
  'centos7-x86_64-unknown-linux-gnu-java': 'centos7-x86_64',
  'windows-i686-pc-windows-msvc-java': 'windows-i686',
  'macos-x86_64-apple-darwin-java': 'macos-x86_64',
  'windows-x86_64-pc-windows-msvc-java': 'windows-x86_64',
  'debian-x86_64-unknown-linux-gnu-java': 'linux-gnu-x86_64',
};

const JAVA_NATIVE_DIR = path.resolve(
  BASE_DIR,
  'statsig-java/src/main/resources/native',
);

export function buildJava(options: BuilderOptions) {
  Log.title(`Building statsig-java`);

  options.release = true; // default to true
  options.targetProject = 'statsig_java';
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
    if (!file.includes(options.os)) {
      return;
    }

    const arch = getArchInfo(options.arch).name;
    const classifier = resolveClassifierFromOsArch(options.os, arch);
    if (!classifier) {
      return;
    }

    const filename = path.basename(file);
    const destDir = path.resolve(JAVA_NATIVE_DIR, classifier);
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

function resolveClassifierFromOsArch(os: string, arch: string): string | null {
  const prefix = `${os}-${arch}`;
  const matchedEntry = Object.entries(TARGET_MAPPING).find(([key]) =>
    key.startsWith(prefix),
  );

  if (matchedEntry) {
    return matchedEntry[1];
  }

  return null;
}
