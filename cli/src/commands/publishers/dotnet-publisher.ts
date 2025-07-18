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
  await publishDotnetPackages(options);
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

async function publishDotnetPackages(options: PublisherOptions) {
  Log.stepBegin('Publishing Dotnet Packages');

  ensureEmptyDir(NUPKG_DIR);

  const nativeProjs = [
        'src/Native/Statsig.NativeAssets.linux-x64.csproj',
        'src/Native/Statsig.NativeAssets.osx-arm64.csproj',
        'src/Native/Statsig.NativeAssets.osx-x86.csproj',
        'src/Native/Statsig.NativeAssets.win-x64.csproj',
        'src/Native/Statsig.NativeAssets.win-x86.csproj',
        'src/Native/Statsig.NativeAssets.linux-arm64.csproj',
        // Add more native projects as needed
    ];

  for (const proj of nativeProjs) {
    packProject(proj);
  }
  Log.stepProgress('Packed all native projects, starting to push all native packages');

  const nupkgs = fs.readdirSync(NUPKG_DIR).filter(f => f.endsWith('.nupkg'));
  const nativePkgs = nupkgs.filter(name => name.includes('NativeAssets'));

  const version = getVersionFromNupkgList(nativePkgs);
  Log.stepProgress(`Detected version: ${version}`);

  for (const pkg of nativePkgs) {
    pushNupkg(pkg);
  }

  Log.stepProgress('Finished pushing native packages. Waiting for all native packages to be indexed...');
  const allIndexed = await waitForPackagesIndexed(nativePkgs, version);

  if (!allIndexed) {
    throw new Error('Timeout waiting for native packages to be indexed');
  }

  packProject('src/Statsig/Statsig.csproj');
  Log.stepProgress('Packed statsig(main) project, starting to push main packages');
  
  const updatedNupkgs = fs.readdirSync(NUPKG_DIR).filter(f => f.endsWith('.nupkg'));
  const mainPkgs = updatedNupkgs.filter(name => !name.includes('NativeAssets'));

  if (mainPkgs.length !== 1) {
    throw new Error(`Expected exactly one main package, found ${mainPkgs.length}: ${mainPkgs.join(', ')}`);
  }

  pushNupkg(mainPkgs[0]);

  Log.stepEnd('Dotnet packages published successfully');
}

function pushNupkg(fileName: string) {
  const fullPath = path.join(NUPKG_DIR, fileName);
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

// Get current version from nupkg files
function getVersionFromNupkgList(nupkgs: string[]): string {
  for (const file of nupkgs) {
    const match = file.match(/.+\.(\d+\.\d+\.\d+(?:-[\w\d.]+)?)\.nupkg$/);
    if (match) {
      return match[1]; // "0.x.x-beta.x"
    }
  }
  throw new Error('Could not determine version from nupkg filenames');
}

// Check all NATIVE packages are indexed on NuGet
async function waitForPackagesIndexed(
  pkgs: string[],
  version: string,
  timeoutMs: number = 600_000,
  intervalMs: number = 10_000
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  const pending = new Set<string>(pkgs);

  Log.stepProgress(`⏳ Waiting for NuGet to index ${pending.size} packages (version ${version})...`);

  while (Date.now() < deadline && pending.size > 0) {
    for (const pkg of [...pending]) {
      const name = pkg.replace(/\.\d+\.\d+\.\d+(?:-[\w\d.]+)?\.nupkg$/, '');
      const url = `https://api.nuget.org/v3-flatcontainer/${name.toLowerCase()}/index.json`;

      try {
        const res = await fetch(url);
        if (!res.ok) {
          Log.stepProgress(`❌ Failed to fetch index.json for ${name}: ${res.status}`);
          return false;
        }

        const data = await res.json();
        const exists = data.versions?.some((v: string) => v.toLowerCase() === version.toLowerCase());

        if (exists) {
          Log.stepProgress(`✅ FlatContainer indexed: ${name}@${version}`);
          pending.delete(pkg);
        } else {
          Log.stepProgress(`⌛ Not yet indexed: ${name}@${version}`);
        }
      } catch (e) {
        Log.stepProgress(`⚠️ Error checking ${name}: ${e}`);
      }
    }

    if (pending.size > 0) {
      Log.stepProgress(`⌛ Still waiting for: ${[...pending].join(', ')}`);
      await new Promise(res => setTimeout(res, intervalMs));
    }
  }

  if (pending.size === 0) {
    Log.stepEnd(`✅ All native packages indexed on NuGet`);
    return true;
  } else {
    Log.stepProgress(`❌ Timeout: Not all native packages were indexed: ${[...pending].join(', ')}`);
    return false;
  }
}
