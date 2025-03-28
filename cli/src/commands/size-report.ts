import {
  ensureEmptyDir,
  getFileSize,
  getHumanReadableSize,
  getRelativePath,
  getRootedPath,
  listFiles,
  unzipFiles,
} from '@/utils/file_utils.js';
import {
  downloadWorkflowRunArtifacts,
  getOctokit,
  getWorkflowRun,
  getWorkflowRunArtifacts,
} from '@/utils/octokit_utils.js';
import {
  SizeCommentWithId,
  SizeInfo,
  fetchPreviousSizeInfo,
} from '@/utils/size_report_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { Octokit } from 'octokit';

import { CommandBase, OptionConfig } from './command_base.js';

const REPORT_SIZE_ISSUE_NUMBER = 159;
const PRIVATE_REPO = 'private-statsig-server-core';

type SizeReportOptions = {
  shouldPersist: boolean;
  workflowId: string;
  repository: string;
  workingDir: string;
  skipArtifactDownload: boolean;
  disregardWorkflowChecks: boolean;
};

export class SizeReport extends CommandBase {
  constructor() {
    const options: OptionConfig[] = [
      {
        flags: '--should-persist <boolean>',
        description: 'Whether to persist the size report to the Github issue',
        required: true,
      },
      {
        flags: '-w, --workflow-id <string>',
        description: 'The Github workflow run the contains the build artifacts',
        required: true,
      },
      {
        flags: '-r, --repository <string>',
        description: 'The repository to use',
        required: true,
      },
      {
        flags: '-wd, --working-dir <string>',
        description: 'The working directory to use',
        defaultValue: '/tmp/statsig-server-core-size-report',
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
      description: 'Reports on the binary size of the ffi as PR comments',
    });
  }

  override async run(options: SizeReportOptions) {
    Log.title('Size Report');

    options.shouldPersist = (options.shouldPersist as any) === 'true';
    options.workingDir = getRelativePath(options.workingDir);

    Log.info(`SizeReportOptions: ${JSON.stringify(options, null, 2)}\n`);

    if (!options.skipArtifactDownload) {
      ensureEmptyDir(options.workingDir);

      const octokit = await getOctokit();
      await getWorkflowRun(octokit, options);
      const analyzeOpts = {
        ...options,
        package: 'analyze',
      };
      const artifacts = await getWorkflowRunArtifacts(octokit, analyzeOpts);
      await downloadWorkflowRunArtifacts(
        octokit,
        analyzeOpts,
        artifacts.artifacts,
      );

      const zipFiles = listFiles(options.workingDir, '*.zip');
      unzipFiles(zipFiles, options.workingDir);
    }

    const workingDir = options.workingDir;
    const files = [
      ...listFiles(workingDir, '**/target/**/release/*.dylib'),
      ...listFiles(workingDir, '**/target/**/release/*.so'),
      ...listFiles(workingDir, '**/target/**/release/*.dll'),
      ...listFiles(workingDir, '**/statsig-node/build/*.node'),
      ...listFiles(workingDir, '**/statsig-pyo3/build/*.whl'),
    ];

    const sizes: SizeInfo[] = files.map((file) => {
      const fullPath = getRootedPath(file);
      const bytes = getFileSize(fullPath);
      const size = getHumanReadableSize(fullPath);

      return { path: getShortPath(fullPath), size, file, bytes };
    });

    Log.stepBegin('Listing sizes');
    sizes.forEach((size) => {
      Log.stepProgress(`${size.path} - ${size.size}`);
    });
    Log.stepEnd('All sizes listed');

    const octokit = await getOctokit();

    if (options.shouldPersist) {
      await persistSizesToGithubIssue(octokit, sizes);
    } else {
      const pr = await getPullRequestFromBranch(octokit);

      if (!pr) {
        Log.stepEnd('No pull request found, skipping...', 'failure');
        return;
      }

      await reportSizesToPullRequest(octokit, pr.number, sizes);
    }
  }
}

async function persistSizesToGithubIssue(octokit: Octokit, sizes: SizeInfo[]) {
  Log.stepBegin('Fetching previous size info');
  const { result: prevSizeComments, error } =
    await fetchPreviousSizeInfo(octokit);

  if (error || !prevSizeComments) {
    Log.stepEnd('Failed to fetch previous size info', 'failure');
    throw error;
  }

  const entries = Object.entries(prevSizeComments);
  entries.forEach(([key, value]) => {
    Log.stepProgress(`${key} - ${value.size}`);
  });

  Log.stepEnd(`Previous size info fetched`);

  Log.stepBegin('Upserting comments');
  await Promise.all(
    sizes.map(async (size) => {
      const previous = prevSizeComments[size.path];
      const result = await upsertCommentOnSizeReportIssue(
        octokit,
        size,
        previous ?? null,
      );
      Log.stepProgress(`${size.path} - ${result.html_url}`);
    }),
  );
  Log.stepEnd('All comments upserted');
}

async function getPullRequestFromBranch(octokit: Octokit) {
  const { data } = await octokit.rest.pulls.list({
    owner: 'statsig-io',
    repo: PRIVATE_REPO,
  });

  return data.find((pr) => {
    return pr.base.ref === 'main' && pr.head.sha === process.env.GITHUB_SHA;
  });
}

async function reportSizesToPullRequest(
  octokit: Octokit,
  prNumber: number,
  sizes: SizeInfo[],
) {
  const { result: prevSizeComments, error } =
    await fetchPreviousSizeInfo(octokit);

  if (error || !prevSizeComments) {
    Log.stepEnd('Failed to fetch previous size info', 'failure');
    throw error;
  }

  const { data } = await octokit.rest.issues.listComments({
    owner: 'statsig-io',
    repo: PRIVATE_REPO,
    issue_number: prNumber,
  });

  const comment = data.find(
    (comment) =>
      comment.user.login === 'statsig-kong[bot]' &&
      comment.body?.includes('## 📦 Size Report'),
  );

  const body = getFormattedSizeReport(sizes, prevSizeComments);

  if (comment) {
    await octokit.rest.issues.updateComment({
      owner: 'statsig-io',
      repo: PRIVATE_REPO,
      comment_id: comment.id,
      body,
    });

    return data;
  } else {
    const { data } = await octokit.rest.issues.createComment({
      owner: 'statsig-io',
      repo: PRIVATE_REPO,
      issue_number: prNumber,
      body,
    });

    return data;
  }
}

async function upsertCommentOnSizeReportIssue(
  octokit: Octokit,
  size: SizeInfo,
  previous: SizeCommentWithId | null,
) {
  const comment = {
    path: size.path,
    bytes: size.bytes,
    size: size.size,
    commit: process.env.GITHUB_SHA ?? 'UNKNOWN',
    workflow_run_id: process.env.GITHUB_RUN_ID ?? 'UNKNOWN',
    comment_id: previous?.comment_id,
  };

  const body = `\`\`\`json\n${JSON.stringify(comment, null, 2)}\n\`\`\``;

  if (previous) {
    const { data } = await octokit.rest.issues.updateComment({
      owner: 'statsig-io',
      repo: PRIVATE_REPO,
      comment_id: comment.comment_id,
      body,
    });

    return data;
  } else {
    const { data } = await octokit.rest.issues.createComment({
      owner: 'statsig-io',
      repo: PRIVATE_REPO,
      issue_number: REPORT_SIZE_ISSUE_NUMBER,
      body,
    });

    return data;
  }
}

function getFormattedSizeReport(
  currentSizes: SizeInfo[],
  previousSizes: Record<string, SizeCommentWithId>,
) {
  const lines = [
    '## 📦 Size Report',
    '| File | Size | % Change |',
    '|--|--|--|',
  ];

  const previousSizeEntries = Object.entries(previousSizes);

  currentSizes.forEach((size) => {
    const found = previousSizeEntries.find(([key, _]) =>
      size.path.includes(key.slice(1) /* remove leading . */),
    );

    if (found) {
      const change = ((size.bytes - found[1].bytes) / found[1].bytes) * 100;
      lines.push(
        `| ${getStylizedPath(found[0])} | ${size.size} | ${getStylizedPercentageChange(change)} |`,
      );
    } else {
      lines.push(
        `| ${getStylizedPath(size.path)} | ${size.size} | ERR_NO_PREVIOUS_SIZE |`,
      );
    }
  });

  return lines.join('\n');
}

function getStylizedPath(path: string) {
  if (path.includes('./statsig-pyo3/build/')) {
    return path.replace('./statsig-pyo3/build/', '');
  }

  if (path.includes('./target/')) {
    return path.replace('./target/', '');
  }

  return path;
}

function getStylizedPercentageChange(change: number) {
  if (change > 0) {
    return '${\\color{orangered}⬆️}$ ' + change.toFixed(2) + '%';
  } else if (change < 0) {
    return '${\\color{limegreen}⬇️}$ ' + change.toFixed(2) + '%';
  }

  return '${\\color{royalblue} =}$ No Change';
}

function getShortPath(path: string) {
  const sanePath = path.replace(/\\/g, '/');

  if (sanePath.includes('statsig-pyo3/build/')) {
    const parts = sanePath.split('statsig-pyo3/build/');
    return parts[1].replace(/-\d+\.\d+\.\d+[a-z0-9]*/, '');
  }

  if (sanePath.includes('statsig-node/build/')) {
    const parts = sanePath.split('statsig-node/build/');
    return parts[1];
  }

  if (sanePath.includes('/target/')) {
    const parts = sanePath.split('/target/');
    return parts[1];
  }

  return sanePath;
}
