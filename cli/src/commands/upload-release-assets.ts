import {
  ensureEmptyDir,
  getRootedPath,
  listFiles,
  zipFile,
} from '@/utils/file_utils.js';
import {
  deleteReleaseAssetWithName,
  getOctokit,
  getReleaseByVersion,
  uploadReleaseAsset,
} from '@/utils/octokit_utils.js';
import { SemVer } from '@/utils/semver.js';
import { Log } from '@/utils/teminal_utils.js';
import path from 'node:path';

import { CommandBase } from './command_base.js';

type Options = {
  releaseId: string;
  target: string;
  repository: string;
};

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
    'libstatsig_ffi.a': 'static',
    'libstatsig_ffi.dll': 'shared',
  },
  'i686-pc-windows-msvc': {
    'libstatsig_ffi.a': 'static',
    'libstatsig_ffi.dll': 'shared',
  },
};

export class UploadReleaseAssets extends CommandBase {
  constructor() {
    super(import.meta.url, {
      description: 'Uploads release assets to GitHub',
      options: [
        {
          flags: '-i, --release-id <string>',
          description: 'The release ID to upload assets to',
          required: true,
        },
        {
          flags: '-t, --target <string>',
          description: 'The target to upload assets for',
          required: true,
        },
        {
          flags: '-r, --repository <string>',
          description: 'The repository to upload assets to',
          required: true,
        },
      ],
    });
  }

  override async run(options: Options) {
    Log.title('Uploading Release Assets');

    Log.stepBegin('Configuration');
    Log.stepEnd(`Release ID: ${options.releaseId}`);

    const assetMapping = ASSET_MAPPING[options.target];
    if (!assetMapping) {
      throw new Error(`No mapping found for target: ${options.target}`);
    }

    Log.stepBegin('Finding asset to upload');
    const files: { assetPath: string; type: string }[] = [];
    const assets = Object.keys(assetMapping);

    for (const asset of assets) {
      const found = listFiles(getRootedPath('target/release'), asset, {
        maxDepth: 1,
      });

      if (found.length === 0) {
        Log.stepProgress(`No file found for asset: ${asset}`, 'failure');
        continue;
      }

      if (found.length > 1) {
        Log.stepProgress(`Multiple files found for asset: ${asset}`, 'failure');
        found.forEach((file) => {
          Log.stepProgress(`-- Found: ${file}`, 'failure');
        });
        continue;
      }

      const type = assetMapping[asset];
      Log.stepProgress(`Found: ${found[0]} (${type})`);
      files.push({
        assetPath: found[0],
        type,
      });
    }

    if (files.length !== assets.length) {
      Log.stepEnd('Finished finding assets', 'failure');
      process.exit(1);
    }

    Log.stepEnd('Finished finding assets');

    Log.stepBegin('Compressing files');

    ensureEmptyDir('/tmp/statsig-core-assets');

    const compressedFiles: string[] = [];
    for (const file of files) {
      const outpath = `/tmp/statsig-core-assets/statsig-core-${options.target}-${file.type}.zip`;
      zipFile(file.assetPath, outpath);
      compressedFiles.push(outpath);
      Log.stepProgress(`Compressed ${outpath}`);
    }
    Log.stepEnd('Finished compressing files');

    Log.stepBegin('Uploading files');
    const octokit = await getOctokit();

    for (const file of compressedFiles) {
      const didDelete = await deleteReleaseAssetWithName(
        octokit,
        options.repository,
        parseInt(options.releaseId),
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
        parseInt(options.releaseId),
        file,
        path.basename(file),
      );

      if (error) {
        Log.stepProgress(`Failed to upload ${file}`, 'failure');
        Log.stepProgress(`Error: ${error}`, 'failure');
      }
    }
    Log.stepEnd('Finished uploading files');

    Log.conclusion(`Successfully Uploaded Release Assets`);
  }
}
