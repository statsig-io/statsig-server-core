import { createAppAuth } from '@octokit/auth-app';
import { createReadStream } from 'fs';
import { Octokit } from 'octokit';
import path from 'path';

import { getFileSize } from './file_utils.js';
import { SemVer } from './semver.js';

const GITHUB_APP_ID = process.env.GH_APP_ID;
const GITHUB_INSTALLATION_ID = process.env.GH_APP_INSTALLATION_ID;
const GITHUB_APP_PRIVATE_KEY = process.env.GH_APP_PRIVATE_KEY;

export type GhRelease = Awaited<
  ReturnType<Octokit['rest']['repos']['getReleaseByTag']>
>['data'];

export type GhAsset = Awaited<
  ReturnType<Octokit['rest']['repos']['listReleaseAssets']>
>['data'][number];

type GhBranch = Awaited<ReturnType<Octokit['rest']['git']['getRef']>>['data'];

export async function getOctokit() {
  const token = await getInstallationToken();

  return new Octokit({
    auth: token,
  });
}

export async function getInstallationToken() {
  if (!GITHUB_APP_ID) {
    throw new Error('GITHUB_APP_ID is not set');
  }

  if (!GITHUB_INSTALLATION_ID) {
    throw new Error('GITHUB_INSTALLATION_ID is not set');
  }

  if (!GITHUB_APP_PRIVATE_KEY) {
    throw new Error('GITHUB_APP_PRIVATE_KEY is not set');
  }

  const auth = createAppAuth({
    appId: GITHUB_APP_ID,
    privateKey: GITHUB_APP_PRIVATE_KEY,
  });

  const result = await auth({
    type: 'installation',
    installationId: GITHUB_INSTALLATION_ID,
  });

  return result.token;
}

export async function getReleaseByVersion(
  octokit: Octokit,
  repo: string,
  version: SemVer,
): Promise<GhRelease | null> {
  try {
    const { data } = await octokit.rest.repos.getReleaseByTag({
      owner: 'statsig-io',
      repo,
      tag: version.toString(),
    });

    return data;
  } catch {
    return null;
  }
}

export async function getBranchByVersion(
  octokit: Octokit,
  repo: string,
  version: SemVer,
): Promise<GhBranch | null> {
  try {
    const branch = version.toBranch();
    const branchRef = `heads/${branch}`;

    const result = await octokit.rest.git.getRef({
      owner: 'statsig-io',
      repo,
      ref: branchRef,
    });

    return result.data;
  } catch {
    return null;
  }
}

export async function deleteReleaseAssetWithName(
  octokit: Octokit,
  repo: string,
  releaseId: number,
  assetName: string,
) {
  const { data } = await octokit.rest.repos.listReleaseAssets({
    owner: 'statsig-io',
    repo,
    release_id: releaseId,
    per_page: 100,
  });

  const existingAsset = data.find((asset) => asset.name === assetName);

  if (!existingAsset) {
    return false;
  }

  await octokit.rest.repos.deleteReleaseAsset({
    owner: 'statsig-io',
    repo,
    asset_id: existingAsset.id,
  });

  return true;
}

export async function uploadReleaseAsset(
  octokit: Octokit,
  repo: string,
  releaseId: number,
  assetPath: string,
) {
  const assetContent = createReadStream(assetPath);
  const size = getFileSize(assetPath);

  try {
    const response = await octokit.rest.repos.uploadReleaseAsset({
      owner: 'statsig-io',
      repo,
      release_id: releaseId,
      name: path.basename(assetPath),
      // It wants a string, but it works with streams too
      data: assetContent as unknown as string,
      headers: {
        'Content-Length': size.toString(),
      },
    });

    return { result: response.data, error: null };
  } catch (error) {
    return { result: null, error };
  }
}

export async function createReleaseForVersion(
  octokit: Octokit,
  repo: string,
  version: SemVer,
  targetSha: string,
): Promise<GhRelease | null> {
  try {
    const result = await octokit.rest.repos.createRelease({
      owner: 'statsig-io',
      repo,
      tag_name: version.toString(),
      target_commitish: targetSha,
      prerelease: version.isBeta(),
    });

    return result.data;
  } catch {
    return null;
  }
}

export async function getAllAssetsForRelease(
  octokit: Octokit,
  repo: string,
  releaseId: number,
  prefix: string,
) {
  try {
    const { data } = await octokit.rest.repos.listReleaseAssets({
      owner: 'statsig-io',
      repo,
      release_id: releaseId,
      per_page: 100,
    });

    const assets = data.filter((asset) => asset.name.startsWith(prefix));

    return { assets, error: null };
  } catch (error) {
    return { error, assets: null };
  }
}

export async function downloadReleaseAsset(
  octokit: Octokit,
  repo: string,
  assetId: number,
): Promise<ArrayBuffer> {
  const file = await octokit.rest.repos.getReleaseAsset({
    owner: 'statsig-io',
    repo,
    asset_id: assetId,
    headers: {
      Accept: 'application/octet-stream',
    },
  });

  // the 'Accept' header means it returns a buffer
  return file.data as unknown as ArrayBuffer;
}
