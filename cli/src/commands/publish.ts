import { ensureEmptyDir } from '@/utils/file_utils.js';
import { downloadArtifactToFile, getOctokit } from '@/utils/octokit_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import type { RestEndpointMethodTypes } from '@octokit/plugin-rest-endpoint-methods';
import { Octokit } from 'octokit';

import { CommandBase, OptionConfig } from './command_base.js';
import { ffiPublish } from './publishers/ffi-publisher.js';
import { nodePublish } from './publishers/node-publisher.js';
import {
  PACKAGES,
  Package,
  PublisherOptions,
} from './publishers/publisher-options.js';
import { publishPython } from './publishers/python-publish.js';

const PUBLISHERS: Record<
  Package,
  (options: PublisherOptions) => Promise<void>
> = {
  python: publishPython,
  node: nodePublish,
  ffi: ffiPublish,
};

type GHArtifact =
  RestEndpointMethodTypes['actions']['listWorkflowRunArtifacts']['response']['data']['artifacts'][number];

export class Publish extends CommandBase {
  constructor() {
    const options: OptionConfig[] = [
      {
        flags: '-w, --workflow-id <string>',
        description: 'The Github workflow run the contains the build artifacts',
        required: true,
      },
      {
        flags: '-p, --package <string>',
        description: 'The package to publish',
        choices: PACKAGES,
        required: true,
      },
      {
        flags: '-r, --repository <string>',
        description: 'The repository to publish to',
        required: true,
        defaultValue: 'private-statsig-server-core',
      },
      {
        flags: '-wd, --working-dir <string>',
        description: 'The working directory to use',
        defaultValue: '/tmp/statsig-server-core-publish',
      },
    ];

    super(import.meta.url, {
      options,
      description: 'Publishes the specified package',
    });
  }

  override async run(options: PublisherOptions) {
    Log.title(`Publishing ${options.package}`);

    Log.stepBegin('Configuration');
    Log.stepProgress(`Workflow ID: ${options.workflowId}`);
    Log.stepProgress(`Repository: ${options.repository}`);
    Log.stepEnd(`Package: ${options.package}`);

    ensureEmptyDir(options.workingDir);

    const octokit = await getOctokit();
    const workflowRun = await getWorkflowRun(octokit, options);
    const artifacts = await getWorkflowRunArtifacts(octokit, options);
    const downloadedArtifacts = await downloadWorkflowRunArtifacts(
      octokit,
      options,
      artifacts.artifacts,
    );

    PUBLISHERS[options.package](options);

    Log.conclusion(`Successfully published ${options.package}`);
  }
}

async function getWorkflowRun(octokit: Octokit, options: PublisherOptions) {
  Log.stepBegin(`Getting workflow run ${options.workflowId}`);

  const response = await octokit.rest.actions.getWorkflowRun({
    owner: 'statsig-io',
    repo: options.repository,
    run_id: Number(options.workflowId),
  });

  if (response.status !== 200) {
    throw new Error(`Failed to get workflow run ${options.workflowId}`);
  }

  if (response.data.status !== 'completed') {
    const message = `Workflow run ${options.workflowId} is not completed`;
    Log.stepEnd(message, 'failure');
    throw new Error(message);
  }

  if (response.data.conclusion !== 'success') {
    const message = `Workflow run ${options.workflowId} is not successful`;
    Log.stepEnd(message, 'failure');
    throw new Error(message);
  }

  Log.stepEnd(`Got workflow run ${options.workflowId}`);

  return response.data;
}

async function getWorkflowRunArtifacts(
  octokit: Octokit,
  options: PublisherOptions,
) {
  Log.stepBegin(`Getting workflow run artifacts`);

  const response = await octokit.rest.actions.listWorkflowRunArtifacts({
    owner: 'statsig-io',
    repo: options.repository,
    run_id: Number(options.workflowId),
  });

  if (response.status !== 200) {
    const message = `Failed to get workflow run artifacts`;
    Log.stepEnd(message, 'failure');
    throw new Error(message);
  }

  response.data.artifacts = response.data.artifacts.filter((artifact) => {
    if (artifact.name.endsWith(options.package)) {
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

async function downloadWorkflowRunArtifacts(
  octokit: Octokit,
  options: PublisherOptions,
  artifacts: GHArtifact[],
) {
  Log.stepBegin(`Downloading workflow run artifacts`);

  const responses = await Promise.all(
    artifacts.map(async (artifact) => {
      const zipPath = `/tmp/statsig-server-core-publish/${artifact.name}.zip`;
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
