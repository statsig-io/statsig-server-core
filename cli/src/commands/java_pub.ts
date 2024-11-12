import { ensureEmptyDir, unzip } from '@/utils/file_utils.js';
import {
  GhAsset,
  GhRelease,
  downloadReleaseAsset,
  getAllAssetsForRelease,
  getOctokit,
  getReleaseByVersion,
} from '@/utils/octokit_utils.js';
import { SemVer } from '@/utils/semver.js';
import { Log } from '@/utils/teminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import { Command } from 'commander';
import { Octokit } from 'octokit';

const TEMP_DIR = '/tmp/statsig-java-build';

export class JavaPub extends Command {
  constructor() {
    super('java-pub');

    this.description('Publishes the statsig-java package to Maven');

    this.argument(
      '<repo>',
      'The name of the repository, e.g. private-statsig-server-core',
    );

    this.action(this.run.bind(this));
  }

  async run(repo: string) {
    Log.title('Publishing statsig-java to Maven');

    Log.stepBegin('Configuration');
    Log.stepEnd(`Repo: ${repo}`);

    const version = getRootVersion();
    const octokit = await getOctokit();

    ensureEmptyDir(TEMP_DIR);

    const release = await getRelease(octokit, repo, version);
    const assets = await getStatsigLibAssets(octokit, repo, release);

    await downloadAndUnzipAssets(octokit, repo, assets);
  }
}

async function getRelease(octokit: Octokit, repo: string, version: SemVer) {
  Log.stepBegin('Getting release');
  const release = await getReleaseByVersion(octokit, repo, version);
  if (!release) {
    Log.stepEnd('Release not found', 'failure');
    process.exit(1);
  }
  Log.stepEnd(`Release Found: ${release.html_url}`);

  return release;
}

async function getStatsigLibAssets(
  octokit: Octokit,
  repo: string,
  release: GhRelease,
) {
  Log.stepBegin('Getting all assets for release');

  const { assets, error } = await getAllAssetsForRelease(
    octokit,
    repo,
    release.id,
    'statsig-ffi-',
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

async function downloadAndUnzipAssets(
  octokit: Octokit,
  repo: string,
  assets: GhAsset[],
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
    unzip(file.buffer, TEMP_DIR);
    Log.stepProgress(`Completed: ${file.name}`);
  });

  Log.stepEnd('Unzipped files');
}
