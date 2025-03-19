import {
  BASE_DIR,
  getFileSize,
  getHumanReadableSize,
  getRootedPath,
  listFiles,
} from '@/utils/file_utils.js';
import { getOctokit } from '@/utils/octokit_utils.js';
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
  isRelease: boolean;
};

export class SizeReport extends CommandBase {
  constructor() {
    const options: OptionConfig[] = [
      {
        flags: '--is-release <boolean>',
        description: 'Whether the build is for a release',
        required: true,
      },
    ];

    super(import.meta.url, {
      options,
      description: 'Reports on the binary size of the ffi as PR comments',
    });
  }

  override async run(options: SizeReportOptions) {
    Log.title('Size Report');

    options.isRelease = (options.isRelease as any) === 'true';

    Log.info(`SizeReportOptions: ${JSON.stringify(options, null, 2)}\n`);

    const files = [
      ...listFiles(BASE_DIR, '**/target/**/release/*.dylib'),
      ...listFiles(BASE_DIR, '**/target/**/release/*.so'),
      ...listFiles(BASE_DIR, '**/target/**/release/*.dll'),
      ...listFiles(BASE_DIR, '**/statsig-node/build/*.node'),
      ...listFiles(BASE_DIR, '**/statsig-pyo3/build/*.whl'),
    ];

    const sizes: SizeInfo[] = files.map((file) => {
      const fullPath = getRootedPath(file);
      const bytes = getFileSize(fullPath);
      const size = getHumanReadableSize(fullPath);

      return { path: fullPath.replace(BASE_DIR, '.'), size, file, bytes };
    });

    Log.stepBegin('Listing sizes');
    sizes.forEach((size) => {
      Log.stepProgress(`${size.path} - ${size.size}`);
    });
    Log.stepEnd('All sizes listed');

    const octokit = await getOctokit();

    if (options.isRelease) {
      await persistSizesToGithubIssue(octokit, sizes);
    } else {
      const pr = await getPullRequestFromBranch(octokit);

      if (!pr) {
        Log.stepEnd('No pull request found', 'failure');
        process.exit(1);
      }

      // Skip for now. I need to update it to download all artifacts and run all together
      // await reportSizesToPullRequest(octokit, pr.number, sizes);
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
  const repo = process.env.GITHUB_REPOSITORY;
  if (!repo) {
    throw new Error('GITHUB_REPOSITORY is not set');
  }

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

  const body = getFormattedSizeReport(comment?.body, sizes);

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

function getFormattedSizeReport(current: string | null, sizes: SizeInfo[]) {
  const lines = [
    '## 📦 Size Report',
    '| File | Size | % Change |',
    '|--|--|--|',
  ];

  sizes.forEach((size) => {
    lines.push(`| ${size.path} | ${size.size} | ${size.bytes} |`);
  });

  return lines.join('\n');
}
