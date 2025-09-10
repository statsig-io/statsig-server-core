import {
  GhAsset,
  GhRelease,
  downloadReleaseAsset,
  getAllAssetsForRelease,
  getReleaseByVersion,
} from '@/utils/octokit_utils.js';
import {
  ensureEmptyDir,
  getRootedPath,
  listFiles,
  unzip,
  zipFile,
} from '@/utils/file_utils.js';

import { Log } from '@/utils/terminal_utils.js';
import { Octokit } from 'octokit';
import { SemVer } from '@/utils/semver.js';
import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { getRootVersion } from './toml_utils.js';
import path from 'node:path';

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
  'debian-x86_64-unknown-linux-gnu': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  'debian-aarch64-unknown-linux-gnu': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  'centos7-x86_64-unknown-linux-gnu': {
    'libstatsig_ffi.so': 'shared',
    'libstatsig_ffi.so.sig': 'signature',
  },
  'centos7-aarch64-unknown-linux-gnu': {
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
};

type AssetConfig = {
  target?: string;
  skipCompression?: boolean;
  assetName: string;
  file: string;
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
      execSync(`cp ${config.file} ${outpath}`);
      Log.stepProgress(`Copied ${outpath}`);
    } else {
      zipFile(config.file, outpath);
      Log.stepProgress(`Compressed ${outpath}`);
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
  const mappedAssets: AssetConfig[] = [...signatures, ...binaries].map((file) => {
    const found = targets.find((t) => file.includes(t));
    if (!found) {
      Log.stepProgress(`No matching asset found for ${file}`, 'failure');
      allAssetsMapped = false;
      return null;
    }

    const mapping = ASSET_MAPPING[found];
    const assetName = getAssetName(version, file, found, mapping);

    Log.stepProgress(`Found: ${assetName} -> ${path.basename(file)}`);

    return {
      target: found,
      assetName,
      file,
    };
  });

  const includeFile = getRootedPath('statsig-ffi/include/statsig_ffi.h');
  if (existsSync(includeFile)) {
    mappedAssets.push({
      assetName: 'statsig_ffi.h',
      file: includeFile,
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

function getAssetName(
  version: string,
  file: string,
  target: string,
  mapping: Record<string, string>,
) {
  const keys = Object.keys(mapping);
  const found = keys.find((key) => file.includes(key));
  if (!found) {
    throw new Error(`No matching asset found for ${file}`);
  }

  const type = mapping[found];

  return `statsig-ffi-${version}-${target}-${type}.zip`;
}
