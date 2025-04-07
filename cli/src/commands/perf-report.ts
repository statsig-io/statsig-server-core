import { getOctokit, getWorkflowRunArtifacts } from '@/utils/octokit_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import AdmZip from 'adm-zip';
import { Octokit } from 'octokit';

import { CommandBase } from './command_base.js';

const REPORT_PERF_ISSUE_COMMENT_ID = 2784208652;

type Options = {
  workflowRunId: string;
  shouldPersist: boolean;
};

export class PerfReport extends CommandBase {
  constructor() {
    super(import.meta.url, {
      description: 'Keeps track of the performance of Statsig',
      options: [
        {
          flags: '--workflow-run-id <string>',
          description: 'The workflow run id to download test artifacts from',
          required: true,
        },
        {
          flags: '--should-persist <boolean>',
          description: 'Whether to persist the perf report',
          required: true,
        },
      ],
    });
  }

  override async run(options: Options) {
    Log.title('Persisting Perf Changes');
    options.shouldPersist = (options.shouldPersist as any) === 'true';

    Log.info(`Options: ${JSON.stringify(options, null, 2)}\n`);

    const octokit = await getOctokit();

    const artifacts = await getWorkflowRunArtifacts(octokit, {
      workflowId: options.workflowRunId,
      repository: 'private-statsig-server-core',
      package: 'analyze',
    });

    const found = artifacts.artifacts.find((artifact) =>
      artifact.name.includes('rust-test-outputs'),
    );

    if (!found) {
      Log.stepEnd('No perf report found', 'failure');
      return;
    }

    if (!options.shouldPersist) {
      Log.title('Skipping. Perf commenting is not yet implemented');
    } else {
      await persistPerfReport(octokit, found.id);
    }

    Log.conclusion('Successfully reported on perf changes');
  }
}

async function persistPerfReport(octokit: Octokit, artifactId: number) {
  Log.stepBegin('Persisting perf report');

  const { data } = await octokit.rest.actions.downloadArtifact({
    owner: 'statsig-io',
    repo: 'private-statsig-server-core',
    artifact_id: artifactId,
    archive_format: 'zip',
  });

  const zip = new AdmZip(Buffer.from(data as any));
  const entry = zip.getEntry('test_all_gate_checks_perf.json');
  const json = JSON.parse(entry.getData().toString());

  await octokit.rest.issues.updateComment({
    owner: 'statsig-io',
    repo: 'private-statsig-server-core',
    comment_id: REPORT_PERF_ISSUE_COMMENT_ID,
    body: `\`\`\`json\n${JSON.stringify(json, null, 2)}`,
  });

  Log.stepEnd('Persisted perf report');
}
