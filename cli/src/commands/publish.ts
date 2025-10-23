import {
  ensureEmptyDir,
  getRelativePath,
  listFiles,
  unzipFiles,
} from '@/utils/file_utils.js';
import {
  downloadWorkflowRunArtifacts,
  getOctokit,
  getWorkflowRun,
  getWorkflowRunArtifacts,
} from '@/utils/octokit_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';

import { CommandBase, OptionConfig } from './command_base.js';
import { analyze } from './publishers/analyze.js';
import { dotnetPublish } from './publishers/dotnet-publisher.js';
import { publishElixir } from './publishers/elixir-publisher.js';
import { ffiPublish } from './publishers/ffi-publisher.js';
import { publishGo } from './publishers/go-publisher.js';
import { javaPublish } from './publishers/java-publisher.js';
import { nodePublish } from './publishers/node-publisher.js';
import { publishPhp } from './publishers/php-publisher.js';
import { PACKAGES, PublisherOptions } from './publishers/publisher-options.js';
import { publishPython } from './publishers/python-publish.js';
import { publishCpp } from './publishers/cpp-publisher.js';

const PUBLISHERS: Record<string, (options: PublisherOptions) => Promise<void>> =
  {
    python: publishPython,
    node: nodePublish,
    ffi: ffiPublish,
    java: javaPublish,
    php: publishPhp,
    dotnet: dotnetPublish,
    elixir: publishElixir,
    go: publishGo,
    cpp: publishCpp,
    analyze,
  };

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
        choices: [...PACKAGES, 'analyze'],
        required: true,
      },
      {
        flags: '-r, --repository <string>',
        description: 'The repository to publish to',
        required: true,
        defaultValue: 'private-statsig-server-core',
      },
      {
        flags: '-ri, --release-id <string>',
        description: 'The release id to publish to',
        required: true,
      },
      {
        flags: '-wd, --working-dir <string>',
        description: 'The working directory to use',
        defaultValue: '/tmp/statsig-server-core-publish',
      },
      {
        flags: '-sa, --skip-artifact-download',
        description: 'Skip downloading artifacts',
        defaultValue: false,
      },
      {
        flags: '--disregard-workflow-checks',
        description: 'Whether to disregard workflow checks',
        defaultValue: false,
      },
    ];

    super(import.meta.url, {
      options,
      description: 'Publishes the specified package',
    });
  }

  override async run(options: PublisherOptions) {
    options.workingDir = getRelativePath(options.workingDir);
    options.releaseId = parseInt(options.releaseId + '');

    const version = getRootVersion();

    Log.title(`Publishing ${options.package}`);
    Log.stepBegin('Configuration');
    Log.stepProgress(`Workflow ID: ${options.workflowId}`);
    Log.stepProgress(`Release ID: ${options.releaseId}`);
    Log.stepProgress(`Repository: ${options.repository}`);
    Log.stepProgress(`Is Beta: ${version.isBeta()}`);
    Log.stepProgress(`Working Directory: ${options.workingDir}`);
    Log.stepProgress(
      `Disregard Workflow Checks: ${options.disregardWorkflowChecks}`,
    );
    Log.stepEnd(`Package: ${options.package}`);

    if (!options.skipArtifactDownload) {
      ensureEmptyDir(options.workingDir);

      const octokit = await getOctokit();
      await getWorkflowRun(octokit, options);
      const artifacts = await getWorkflowRunArtifacts(octokit, options);
      await downloadWorkflowRunArtifacts(octokit, options, artifacts.artifacts);

      const zipFiles = listFiles(options.workingDir, '*.zip');
      unzipFiles(zipFiles, options.workingDir);
    }

    await PUBLISHERS[options.package](options);

    Log.conclusion(`Successfully published ${options.package}`);
  }
}
