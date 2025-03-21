import { createAppAuth } from '@octokit/auth-app';
import { RestEndpointMethodTypes } from '@octokit/plugin-rest-endpoint-methods';
import { createReadStream, writeFileSync } from 'fs';
import { Octokit } from 'octokit';
import path from 'path';

import { getFileSize } from './file_utils.js';
import { SemVer } from './semver.js';
import { Log } from './teminal_utils.js';

const GITHUB_APP_ID = process.env.GH_APP_ID;
const GITHUB_INSTALLATION_ID = process.env.GH_APP_INSTALLATION_ID;
const GITHUB_APP_PRIVATE_KEY = process.env.GH_APP_PRIVATE_KEY;

const FFI_BASED_PACKAGES = new Set(['java', 'php', 'ffi']);

export type GhRelease = Awaited<
  ReturnType<Octokit['rest']['repos']['getReleaseByTag']>
>['data'];

export type GhAsset = Awaited<
  ReturnType<Octokit['rest']['repos']['listReleaseAssets']>
>['data'][number];

type GhBranch = Awaited<ReturnType<Octokit['rest']['git']['getRef']>>['data'];
type GHArtifact =
  RestEndpointMethodTypes['actions']['listWorkflowRunArtifacts']['response']['data']['artifacts'][number];

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

export async function createGithubRelease(
  octokit: Octokit,
  repository: string,
  version: SemVer,
  targetSha: string,
) {
  Log.stepBegin('Creating GitHub Release');
  Log.stepProgress(`Repository: ${repository}`);
  Log.stepProgress(`Release Tag: ${version}`);
  Log.stepEnd(`Target SHA: ${targetSha}`);

  Log.stepBegin('Checking for existing release');
  const release = await getReleaseByVersion(octokit, repository, version);

  if (release) {
    Log.stepEnd(`Release already exists: ${release.html_url}`, 'failure');
    process.exit(1);
  }

  Log.stepEnd(`Release ${version} does not exist`);

  Log.stepBegin('Checking if branch exists');
  const branch = await getBranchByVersion(octokit, repository, version);

  if (!branch) {
    Log.stepEnd(`Branch ${version.toBranch()} does not exist`, 'failure');
    process.exit(1);
  }

  Log.stepEnd(`Branch ${branch.ref} exists`);

  Log.stepBegin('Creating release');

  const { result: newRelease, error } = await createReleaseForVersion(
    octokit,
    repository,
    version,
    branch.object.sha,
  );

  if (!newRelease) {
    Log.stepEnd(`Failed to create release`, 'failure');
    console.error(error ?? 'Unknown error');
    process.exit(1);
  }

  Log.stepEnd(`Release created: ${newRelease.html_url}`);

  Log.conclusion(`Successfully Created Release ${version}`);
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
  name?: string,
) {
  const assetContent = createReadStream(assetPath);
  const size = getFileSize(assetPath);

  try {
    const response = await octokit.rest.repos.uploadReleaseAsset({
      owner: 'statsig-io',
      repo,
      release_id: releaseId,
      name: name ?? path.basename(assetPath),
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
  targetSha?: string,
): Promise<{ result?: GhRelease; error?: any }> {
  try {
    const result = await octokit.rest.repos.createRelease({
      owner: 'statsig-io',
      repo,
      tag_name: version.toString(),
      target_commitish: targetSha,
      prerelease: version.isBeta(),
    });

    return { result: result.data };
  } catch (error) {
    console.error(error);
    return { error };
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

export async function downloadArtifactToFile(
  octokit: Octokit,
  repo: string,
  artifactId: number,
  filePath: string,
): Promise<{ data: ArrayBuffer; url: string }> {
  const response = (await octokit.rest.actions.downloadArtifact({
    owner: 'statsig-io',
    repo,
    artifact_id: artifactId,
    archive_format: 'zip',
  })) as { data?: ArrayBuffer; url?: string };

  if (
    !response.data ||
    !response.url ||
    !(response.data instanceof ArrayBuffer)
  ) {
    throw new Error(`Failed to download artifact ${artifactId}`);
  }

  writeFileSync(filePath, Buffer.from(response.data));

  return { data: response.data, url: response.url };
}

export async function getWorkflowRun(
  octokit: Octokit,
  options: {
    workflowId: string;
    repository: string;
    disregardWorkflowChecks: boolean;
  },
) {
  Log.stepBegin(`Getting workflow run ${options.workflowId}`);

  const response = await octokit.rest.actions.getWorkflowRun({
    owner: 'statsig-io',
    repo: options.repository,
    run_id: Number(options.workflowId),
  });

  if (response.status !== 200) {
    throw new Error(`Failed to get workflow run ${options.workflowId}`);
  }

  const canFail = !options.disregardWorkflowChecks;

  if (canFail && response.data.status !== 'completed') {
    const message = `Workflow run ${options.workflowId} is not completed`;
    Log.stepEnd(message, 'failure');
    throw new Error(message);
  }

  if (canFail && response.data.conclusion !== 'success') {
    const message = `Workflow run ${options.workflowId} is not successful`;
    Log.stepEnd(message, 'failure');
    throw new Error(message);
  }

  Log.stepEnd(`Got workflow run ${options.workflowId}`);

  return response.data;
}

export async function getWorkflowRunArtifacts(
  octokit: Octokit,
  options: {
    workflowId: string;
    repository: string;
    package: string;
  },
) {
  Log.stepBegin(`Getting workflow run artifacts`);

  const response = await octokit.rest.actions.listWorkflowRunArtifacts({
    owner: 'statsig-io',
    repo: options.repository,
    run_id: Number(options.workflowId),
    per_page: 100,
  });

  if (response.status !== 200) {
    const message = `Failed to get workflow run artifacts`;
    Log.stepEnd(message, 'failure');
    throw new Error(message);
  }

  Log.stepProgress(`Found ${response.data.artifacts.length} artifacts`);

  response.data.artifacts = response.data.artifacts.filter((artifact) => {
    if (artifact.name.includes('dockerbuild')) {
      return false;
    }

    if (filterArtifact(artifact, options)) {
      Log.stepProgress(`Found: ${artifact.name}`, 'success');
      return true;
    } else {
      Log.stepProgress(`Skipped: ${artifact.name}`);
      return false;
    }
  });

  Log.stepEnd(`Got workflow run artifacts`);

  return response.data;
}

export async function downloadWorkflowRunArtifacts(
  octokit: Octokit,
  options: {
    repository: string;
    package: string;
    workingDir: string;
  },
  artifacts: GHArtifact[],
) {
  Log.stepBegin(`Downloading workflow run artifacts`);

  const responses = await Promise.all(
    artifacts.map(async (artifact) => {
      const zipPath = `${options.workingDir}/${artifact.name}.zip`;
      const response = await downloadArtifactToFile(
        octokit,
        options.repository,
        artifact.id,
        zipPath,
      );

      return { response, artifact, zipPath };
    }),
  );

  let didDownloadAllArtifacts = true;

  responses.forEach(({ response, artifact }) => {
    if (!response.data) {
      const message = `Failed to download artifact ${artifact.name}`;
      Log.stepProgress(message, 'failure');
      didDownloadAllArtifacts = false;
    } else {
      Log.stepProgress(`Downloaded artifact ${artifact.name}`);
    }
  });

  if (!didDownloadAllArtifacts) {
    const message = `Failed to download all artifacts`;
    Log.stepEnd(message, 'failure');
    throw new Error(message);
  }

  Log.stepEnd(`Downloaded workflow run artifacts`);

  return responses;
}

function filterArtifact(artifact: GHArtifact, options: { package: string }) {
  if ((options.package as string) === 'analyze') {
    return true;
  }

  if (artifact.name.endsWith(options.package)) {
    return true;
  }

  if (
    FFI_BASED_PACKAGES.has(options.package) &&
    artifact.name.endsWith('ffi')
  ) {
    return true;
  }

  return false;
}
