import {
  deleteReleaseAssetWithName,
  getOctokit,
  uploadReleaseAsset,
} from '@/utils/octokit_utils.js';
import { mapAssetsToTargets, zipAndMoveAssets } from '@/utils/publish_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

export async function ffiPublish(options: PublisherOptions) {
  const mappedAssets = mapAssetsToTargets(options.workingDir);
  const assetFiles = zipAndMoveAssets(mappedAssets, options.workingDir);

  await uploadAssets(assetFiles, options);

  Log.stepEnd('Finished listing FFI Binaries');
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
