import { getRootedPath } from '@/utils/file_utils.js';
import {
  deleteReleaseAssetWithName,
  getOctokit,
  getReleaseByVersion,
  uploadReleaseAsset,
} from '@/utils/octokit_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import { Command } from 'commander';
import path from 'path';

type Options = {
  repo: string;
  release: string;
};

export class GhAttachAssets extends Command {
  constructor() {
    super('gh-attach-asset');

    this.description('Attaches assets to a release');

    this.requiredOption(
      '--repo, <string>',
      'The name of the repository, e.g. sigstat-php',
    );

    this.argument('<asset-path>', 'The path to the asset to attach');

    this.action(this.run.bind(this));
  }

  async run(asset: string, { repo }: Options) {
    Log.title('Attaching Asset to Release');

    const version = getRootVersion();
    const assetPath = getRootedPath(asset);
    const name = path.basename(assetPath);

    Log.stepBegin('Configuration');
    Log.stepProgress(`Repo: ${repo}`);
    Log.stepProgress(`Release Tag: ${version}`);
    Log.stepProgress(`Asset Name: ${name}`);
    Log.stepEnd(`Asset Path: ${assetPath}`);

    const octokit = await getOctokit();

    Log.stepBegin('Getting release');
    const release = await getReleaseByVersion(octokit, repo, version);
    if (!release) {
      Log.stepEnd('Release not found', 'failure');
      return;
    }
    Log.stepEnd(`Release Found: ${release.html_url}`);

    Log.stepBegin('Deleting existing asset');
    const didDelete = await deleteReleaseAssetWithName(
      octokit,
      repo,
      release.id,
      name,
    );
    if (didDelete) {
      Log.stepEnd('Existing asset deleted');
    } else {
      Log.stepEnd('No existing asset found');
    }

    Log.stepBegin('Uploading asset');
    const uploadUrl = release.upload_url;
    if (!uploadUrl) {
      Log.stepEnd('No upload URL found', 'failure');
      return;
    }

    const { result, error } = await uploadReleaseAsset(
      octokit,
      repo,
      release.id,
      assetPath,
    );

    if (error || !result) {
      const errMessage =
        error instanceof Error ? error.message : error ?? 'Unknown Error';

      Log.stepEnd(`Failed to upload asset: ${errMessage}`, 'failure');
      return;
    }

    Log.stepEnd(`Asset uploaded: ${result.browser_download_url}`);

    console.log('✅ Successfully uploaded asset');
  }
}
