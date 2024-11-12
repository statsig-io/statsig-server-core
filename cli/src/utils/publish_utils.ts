import { unzip } from '@/utils/file_utils.js';
import {
  GhAsset,
  GhRelease,
  downloadReleaseAsset,
  getAllAssetsForRelease,
  getReleaseByVersion,
} from '@/utils/octokit_utils.js';
import { SemVer } from '@/utils/semver.js';
import { Log } from '@/utils/teminal_utils.js';
import { Octokit } from 'octokit';

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
