import {
  ensureEmptyDir,
  getRootedPath,
  listFiles,
  zipFile,
} from '@/utils/file_utils.js';
import {
  deleteReleaseAssetWithName,
  getOctokit,
  uploadReleaseAsset,
} from '@/utils/octokit_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

const ASSET_MAPPING = {
  // macOS
  'aarch64-apple-darwin': {
    'libstatsig_ffi.a': 'static',
    'libstatsig_ffi.dylib': 'shared',
  },
  'x86_64-apple-darwin': {
    'libstatsig_ffi.a': 'static',
    'libstatsig_ffi.dylib': 'shared',
  },
  // Linux GNU
  'x86_64-unknown-linux-gnu': {
    'libstatsig_ffi.a': 'static',
    'libstatsig_ffi.so': 'shared',
  },
  'aarch64-unknown-linux-gnu': {
    'libstatsig_ffi.a': 'static',
    'libstatsig_ffi.so': 'shared',
  },
  // Linux MUSL
  'x86_64-unknown-linux-musl': {
    'libstatsig_ffi.a': 'static',
  },
  'aarch64-unknown-linux-musl': {
    'libstatsig_ffi.a': 'static',
  },
  // Windows
  'x86_64-pc-windows-msvc': {
    'statsig_ffi.lib': 'static',
    'statsig_ffi.dll': 'shared',
  },
  'i686-pc-windows-msvc': {
    'statsig_ffi.lib': 'static',
    'statsig_ffi.dll': 'shared',
  },
};

type AssetConfig = {
  target?: string;
  skipCompression?: boolean;
  assetName: string;
  file: string;
};

export async function ffiPublish(options: PublisherOptions) {
  const mappedAssets = mapAssetsToTargets(options);
  const assetFiles = zipAndMoveAssets(mappedAssets, options);

  await uploadAssets(assetFiles, options);

  Log.stepEnd('Finished listing FFI Binaries');
}

function mapAssetsToTargets(options: PublisherOptions) {
  Log.stepBegin('Mapping Assets to Targets');

  const version = getRootVersion().toString();

  const targets = Object.keys(ASSET_MAPPING);
  const binaries = [
    ...listFiles(options.workingDir, 'target/release/*.a'),
    ...listFiles(options.workingDir, 'target/release/*.dylib'),
    ...listFiles(options.workingDir, 'target/release/*.so'),
    ...listFiles(options.workingDir, 'target/release/*.dll'),
    ...listFiles(options.workingDir, 'target/release/*.lib'),
  ];

  let allAssetsMapped = true;
  const mappedAssets: AssetConfig[] = binaries.map((file) => {
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

function zipAndMoveAssets(
  mappedAssets: ReturnType<typeof mapAssetsToTargets>,
  options: PublisherOptions,
) {
  Log.stepBegin('Zipping Assets');

  const outDir = path.resolve(options.workingDir, 'assets');
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

async function uploadAssets(files: string[], options: PublisherOptions) {
  Log.stepBegin('Uploading files');
  const octokit = await getOctokit();

  for (const file of files) {
    const didDelete = await deleteReleaseAssetWithName(
      octokit,
      options.repository,
      options.releaseId,
      path.basename(file),
    );

    Log.stepProgress(
      didDelete
        ? `Existing asset ${path.basename(file)} was deleted`
        : `No existing asset found for ${path.basename(file)}`,
    );

    Log.stepProgress(`Uploading ${file}`);
    const { error } = await uploadReleaseAsset(
      octokit,
      options.repository,
      options.releaseId,
      file,
      path.basename(file),
    );

    if (error) {
      Log.stepProgress(`Failed to upload ${file}`, 'failure');
      Log.stepProgress(`Error: ${error}`, 'failure');
    }
  }
  Log.stepEnd('Finished uploading files');
}
