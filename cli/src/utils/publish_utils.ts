import {
  ensureEmptyDir,
  getRootedPath,
  listFiles,
  unzip,
  zipDirectory,
  zipFile,
} from '@/utils/file_utils.js';
import {
  GhAsset,
  GhRelease,
  downloadReleaseAsset,
  getAllAssetsForRelease,
  getReleaseByVersion,
} from '@/utils/octokit_utils.js';
import { SemVer } from '@/utils/semver.js';
import { Log } from '@/utils/terminal_utils.js';
import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { Octokit } from 'octokit';

import { getRootVersion } from './toml_utils.js';

const ASSET_MAPPING = {
  // macOS
  'aarch64-apple-darwin': {
    'libstatsig_ffi.dylib': 'shared',
    'libstatsig_ffi.dylib.sig': 'signature',
  },
  'x86_64-apple-darwin': {
    'libstatsig_ffi.dylib': 'shared',
    'libstatsig_ffi.dylib.sig': 'signature',
  },
  // Linux GNU
  'centos7-x86_64-unknown-linux-gnu': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  'centos7-aarch64-unknown-linux-gnu': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  // Linux MUSL
  'alpine-x86_64-unknown-linux-musl': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  'alpine-aarch64-unknown-linux-musl': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  // Windows
  'x86_64-pc-windows-msvc': {
    'statsig_ffi.dll': 'shared',
    'statsig_ffi.dll.sig': 'signature',
  },
  'i686-pc-windows-msvc': {
    'statsig_ffi.dll': 'shared',
    'statsig_ffi.dll.sig': 'signature',
  },
  'aarch64-pc-windows-msvc': {
    'statsig_ffi.dll': 'shared',
    'statsig_ffi.dll.sig': 'signature',
  },
  // Below are deprecated targets, covered by centos7
  'debian-x86_64-unknown-linux-gnu': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  'debian-aarch64-unknown-linux-gnu': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  'amazonlinux2-x86_64-unknown-linux-gnu': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  'amazonlinux2-aarch64-unknown-linux-gnu': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  'amazonlinux2023-x86_64-unknown-linux-gnu': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  'amazonlinux2023-aarch64-unknown-linux-gnu': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
};

type AssetConfig = {
  target?: string;
  skipCompression?: boolean;
  assetName: string;
  files: string[];
};

export async function getRelease(
  octokit: Octokit,
  repo: string,
  version: SemVer,
) {
  Log.stepBegin('Getting release');
  const release = await getReleaseByVersion(octokit, repo, version);
  if (!release) {
    Log.stepEnd('Release not found', 'failure');
    process.exit(1);
  }
  Log.stepEnd(`Release Found: ${release.html_url}`);

  return release;
}

export async function getStatsigLibAssets(
  octokit: Octokit,
  repo: string,
  release: GhRelease,
  assetPrefix: string,
) {
  Log.stepBegin('Getting all assets for release');

  const { assets, error } = await getAllAssetsForRelease(
    octokit,
    repo,
    release.id,
    assetPrefix,
  );

  if (error || !assets) {
    Log.stepEnd('Error getting assets', 'failure');
    console.error(
      error instanceof Error ? error.message : error ?? 'Unknown error',
    );
    process.exit(1);
  }

  if (assets.length === 0) {
    Log.stepEnd('No assets found', 'failure');
    process.exit(1);
  }

  assets.forEach((asset) => {
    Log.stepProgress(`${asset.name}`);
  });

  Log.stepEnd(`Found ${assets.length} assets`);

  return assets;
}

export async function downloadAndUnzipAssets(
  octokit: Octokit,
  repo: string,
  assets: GhAsset[],
  baseTargetDir: string,
  extractWithName?: boolean,
) {
  Log.stepBegin('Downloading assets');

  const files = await Promise.all(
    assets.map(async (asset) => {
      const buffer = await downloadReleaseAsset(octokit, repo, asset.id);
      return { ...asset, buffer };
    }),
  );

  Log.stepEnd(`Downloaded ${files.length} files`);

  Log.stepBegin('Unzipping files');

  files.forEach((file) => {
    const name = file.name.replace('.zip', '');
    const extractTo = extractWithName
      ? baseTargetDir + '/' + name
      : baseTargetDir;
    unzip(file.buffer, extractTo);
    Log.stepProgress(`Completed: ${file.name}`);
  });

  Log.stepEnd('Unzipped files');
}

export function zipAndMoveAssets(
  mappedAssets: ReturnType<typeof mapAssetsToTargets>,
  workingDir: string,
) {
  Log.stepBegin('Zipping Assets');

  const outDir = path.resolve(workingDir, 'assets');
  ensureEmptyDir(outDir);

  const files: string[] = [];
  for (const config of mappedAssets) {
    const outpath = path.resolve(outDir, config.assetName);
    if (config.skipCompression) {
      if (config.files.length === 1) {
        execSync(`cp ${config.files[0]} ${outpath}`);
        Log.stepProgress(`Copied ${outpath}`);
      } else {
        Log.stepProgress(
          `Error: skipCompression only supported for single files`,
          'failure',
        );
        process.exit(1);
      }
    } else {
      if (config.files.length === 1) {
        zipFile(config.files[0], outpath);
        Log.stepProgress(`Compressed ${outpath}`);
      } else {
        // Create a temporary directory to hold all files for this target
        const targetName = config.target || 'unknown';
        if (targetName.includes('..')) {
          Log.stepProgress(`Error: Invalid target name`, 'failure');
          process.exit(1);
        }
        const tempDir = path.resolve(workingDir, `temp_${targetName}`);
        ensureEmptyDir(tempDir);

        config.files.forEach((file) => {
          const fileName = path.basename(file);
          execSync(`cp ${file} ${path.resolve(tempDir, fileName)}`);
        });

        zipDirectory(tempDir, outpath);
        Log.stepProgress(
          `Compressed ${config.files.length} files to ${outpath}`,
        );

        execSync(`rm -rf ${tempDir}`);
      }
    }
    files.push(outpath);
  }

  Log.stepEnd('Finished compressing files');

  return files;
}

export function mapAssetsToTargets(workingDir: string) {
  Log.stepBegin('Mapping Assets to Targets');

  const version = getRootVersion().toString();

  const targets = Object.keys(ASSET_MAPPING);
  const binaries = [
    ...listFiles(workingDir, '**/target/**/release/*.dylib'),
    ...listFiles(workingDir, '**/target/**/release/*.so'),
    ...listFiles(workingDir, '**/target/**/release/*.dll'),
  ];

  const signatures = [
    ...listFiles(workingDir, '**/target/**/release/*.dylib.sig'),
    ...listFiles(workingDir, '**/target/**/release/*.so.sig'),
    ...listFiles(workingDir, '**/target/**/release/*.dll.sig'),
  ];

  Log.stepProgress('signatures:');
  signatures.forEach((signature) => {
    Log.stepProgress(signature);
  });
  Log.stepProgress('binaries:');
  binaries.forEach((binary) => {
    Log.stepProgress(binary);
  });

  let allAssetsMapped = true;
  const mappedAssets: AssetConfig[] = [];

  // Group files by target
  const filesByTarget: Record<string, string[]> = {};
  const allFiles = [...signatures, ...binaries];

  for (const file of allFiles) {
    const found = targets.find((t) => file.includes(t));
    if (!found) {
      Log.stepProgress(`No matching target found for ${file}`, 'failure');
      allAssetsMapped = false;
      continue;
    }

    if (!filesByTarget[found]) {
      filesByTarget[found] = [];
    }
    filesByTarget[found].push(file);
  }

  // Process each target and its associated files
  for (const [target, files] of Object.entries(filesByTarget)) {
    const mapping = ASSET_MAPPING[target];
    const expectedFileNames = Object.keys(mapping);

    // Check if we have all expected files for this target
    const foundFileNames = files.map((file) => path.basename(file));
    const missingFiles = expectedFileNames.filter(
      (expected) => !foundFileNames.some((found) => found === expected),
    );

    if (missingFiles.length > 0) {
      Log.stepProgress(
        `Missing files for target ${target}: ${missingFiles.join(', ')}`,
        'failure',
      );
      allAssetsMapped = false;
      continue;
    }

    // Create asset config for this target with all its files
    const assetName = getAssetNameForTarget(version, target);

    Log.stepProgress(
      `Found target ${target} with files: ${files
        .map((f) => path.basename(f))
        .join(', ')}`,
    );

    mappedAssets.push({
      target,
      assetName,
      files,
    });
  }

  const includeFile = getRootedPath('statsig-ffi/include/statsig_ffi.h');
  if (existsSync(includeFile)) {
    mappedAssets.push({
      assetName: 'statsig_ffi.h',
      files: [includeFile],
      skipCompression: true,
    });
    Log.stepProgress('Found: statsig_ffi.h');
  } else {
    Log.stepProgress('No include file found', 'failure');
    allAssetsMapped = false;
  }

  if (!allAssetsMapped) {
    Log.stepEnd('Failed to map all assets', 'failure');
    process.exit(1);
  }

  Log.stepEnd('Finished mapping assets to targets');

  return mappedAssets;
}

function getAssetNameForTarget(version: string, target: string) {
  return `statsig-ffi-${version}-${target}.zip`;
}
