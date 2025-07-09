import {
  BASE_DIR,
  ensureEmptyDir,
  getRootedPath,
  listFiles,
} from '@/utils/file_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import * as fs from 'fs';
import { execSync } from 'node:child_process';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

const TARGET_MAPPING = {
  'macos-aarch64-apple-darwin-ffi': 'osx-arm64',
  // 'macos-x86_64-apple-darwin-ffi': 'osx-x86',
  // 'windows-i686-pc-windows-msvc-ffi': 'win-x86',
  // 'windows-x86_64-pc-windows-msvc-ffi': 'win-x64',
  // 'debian-aarch64-unknown-linux-gnu-ffi': 'linux-arm64',
  'debian-x86_64-unknown-linux-gnu-ffi': 'linux-x64',
};

const DOTNET_DIR= path.resolve(
  BASE_DIR,
  'statsig-dotnet/runtimes',
);

const NUPKG_DIR = path.resolve(
  BASE_DIR,
  'statsig-dotnet/nupkgs',
);

export async function dotnetPublish(options: PublisherOptions) {
  const libFiles = [
    ...listFiles(options.workingDir, '**/target/**/release/*.dylib'),
    ...listFiles(options.workingDir, '**/target/**/release/*.so'),
    ...listFiles(options.workingDir, '**/target/**/release/*.dll'),
  ].filter(isMappedTarget);

  Log.stepBegin('Clearing Dotnet Native Directory');
  ensureEmptyDir(DOTNET_DIR);
  Log.stepEnd(`Cleared ${DOTNET_DIR}`);

  moveDotnetLibraries(libFiles);
  publishDotnetPackages(options);
}

function moveDotnetLibraries(libFiles: string[]) {
  Log.stepBegin('Moving Dotnet Native Libraries');

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
    const destDir = path.resolve(DOTNET_DIR, destination, 'native');
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

  Log.stepEnd('Successfully moved Dotnet Native Libraries');
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

function isMappedTarget(file: string): boolean {
  return Object.keys(TARGET_MAPPING).some((target) => file.includes(target));
}

function publishDotnetPackages(options: PublisherOptions) {
  Log.stepBegin('Publishing Dotnet Packages');

  ensureEmptyDir(NUPKG_DIR);

  const nativeProjs = [
        'src/Native/Statsig.NativeAssets.linux-x64.csproj',
        'src/Native/Statsig.NativeAssets.osx-arm64.csproj',
        // TODO: Add more
    ];

  for (const proj of nativeProjs) {
    packProject(proj);
  }
  Log.stepProgress('Packed all native projects');

  packProject('src/Statsig/Statsig.csproj');
  Log.stepProgress('Packed statsig project');

  const nupkgs = fs.readdirSync(NUPKG_DIR).filter(f => f.endsWith('.nupkg'));
  const nativePkgs = nupkgs.filter(name => name.includes('NativeAssets'));
  const mainPkgs = nupkgs.filter(name => !name.includes('NativeAssets'));
  
  for (const pkg of nativePkgs) {
    pushNupkg(pkg);
  }

  for (const pkg of mainPkgs) {
    pushNupkg(pkg);
  }

  Log.stepEnd('Dotnet packages published successfully');
}

function pushNupkg(fileName: string) {
  const fullPath = path.join(NUPKG_DIR, fileName);
  // TODO: please change this to a valid NuGet API key
  const pushCommand = `dotnet nuget push ${fullPath} --api-key ${process.env.NUGET_API_KEY} --source https://api.nuget.org/v3/index.json`;

  Log.stepProgress(`Pushing package: ${fileName}`);
  execSync(pushCommand, {
    cwd: getRootedPath('statsig-dotnet'),
    stdio: 'inherit',
  });
}

function packProject(projectPath: string) {
  Log.stepBegin(`Packing Dotnet Project: ${projectPath}`);

  const command = `dotnet pack ${projectPath} -c Release -o ${NUPKG_DIR}`;
  Log.stepProgress(`Running command: ${command}`);

  execSync(command, {
    cwd: getRootedPath('statsig-dotnet'),
    stdio: 'inherit' 
  });
  
  Log.stepEnd(`Successfully packed: ${projectPath}`);
}
