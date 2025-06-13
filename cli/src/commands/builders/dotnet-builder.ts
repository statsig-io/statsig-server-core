import { BuilderOptions } from '@/commands/builders/builder-options.js';
import { getArchInfo } from '@/utils/docker_utils.js';
import { buildFfiHelper } from '@/utils/ffi_utils.js';
import { BASE_DIR, listFiles } from '@/utils/file_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import fs from 'fs';
import { execSync } from 'node:child_process';
import path from 'node:path';

const TARGET_MAPPING = {
  'macos-aarch64-apple-darwin-ffi': 'osx-arm64',
  'macos-x86_64-apple-darwin-ffi': 'osx-x86',
  'windows-i686-pc-windows-msvc-ffi': 'win-x86',
  'windows-x86_64-pc-windows-msvc-ffi': 'win-x64',
  'debian-aarch64-unknown-linux-gnu-ffi': 'linux-arm64',
  'debian-x86_64-unknown-linux-gnu-ffi': 'linux-x64',
};

const DOTNET_DIR= path.resolve(
  BASE_DIR,
  'statsig-dotnet/runtimes',
);

export function buildDotnet(options: BuilderOptions) {
  Log.title(`Building statsig-dotnet`);

  options.release = true; // default to true
  buildFfiHelper(options);
  Log.stepEnd(`Built statsig-dotnet`);

  const libFiles = [
    ...listFiles(BASE_DIR, '**/target/**/release/*.dylib'),
    ...listFiles(BASE_DIR, '**/target/**/release/*.so'),
    ...listFiles(BASE_DIR, '**/target/**/release/*.dll'),
  ];

  moveDotnetLibraries(libFiles, options);
}

function moveDotnetLibraries(libFiles: string[], options: BuilderOptions) {
  Log.stepBegin('Moving Dotnet Libraries');

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
    const destDir = path.resolve(DOTNET_DIR, classifier, 'native');
    ensureDirExists(destDir);

    const destinationPath = path.join(destDir, filename);
    execSync(`cp ${file} ${destinationPath}`);

    Log.stepProgress(`Copied lib to ${destinationPath}`);
    fileMoved = true;
  });

  if (!fileMoved) {
    Log.stepEnd('No matching native files found to move', 'failure');
  } else {
    Log.stepEnd('Successfully copied native files');
  }
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
