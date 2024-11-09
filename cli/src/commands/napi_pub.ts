import {
  downloadReleaseAsset,
  getAllAssetsForRelease,
  getOctokit,
  getReleaseByVersion,
} from '@/utils/octokit_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import AdmZip from 'adm-zip';
import { Command } from 'commander';

export class NapiPub extends Command {
  constructor() {
    super('napi-pub');

    this.description('Publishes the statsig-napi package to NPM');

    this.argument(
      '<repo>',
      'The name of the repository, e.g. private-statsig-server-core',
    );

    this.action(this.run.bind(this));
  }

  async run(repo: string) {
    Log.title('Publishing statsig-napi to NPM');

    Log.stepBegin('Configuration');
    Log.stepEnd(`Repo: ${repo}`);

    const version = getRootVersion();
    const octokit = await getOctokit();

    Log.stepBegin('Getting release');
    const release = await getReleaseByVersion(octokit, repo, version);
    if (!release) {
      Log.stepEnd('Release not found', 'failure');
      process.exit(1);
    }
    Log.stepEnd(`Release Found: ${release.html_url}`);

    Log.stepBegin('Getting all assets for release');
    const { assets, error } = await getAllAssetsForRelease(
      octokit,
      repo,
      release.id,
      'statsig-napi-',
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
      unzip(file.buffer);
      Log.stepProgress(`Completed: ${file.name}`);
    });
    Log.stepEnd('Unzipped files');

    Log.conclusion('Successfully published statsig-napi to NPM');
  }
}

function unzip(buffer: ArrayBuffer) {
  const zip = new AdmZip(Buffer.from(buffer));

  zip.extractAllTo('/tmp/statsig-napi-binaries', false, true);
}
