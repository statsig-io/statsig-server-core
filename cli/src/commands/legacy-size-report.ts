import { covertToHumanReadableSize } from '@/utils/file_utils.js';
import { getOctokit } from '@/utils/octokit_utils.js';
import {
  SizeComment,
  SizeInfo,
  fetchPreviousSizeInfo,
  getFfiBinarySizes,
} from '@/utils/size_report_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { Octokit } from 'octokit';

import { CommandBase } from './command_base.js';

const ONLY_REPORT_ON = [
  'statsig-ffi-aarch64-apple-darwin',
  'statsig-ffi-aarch64-pc-windows-msvc',
  'statsig-ffi-amazonlinux2023-arm64',
];

export class LegacySizeReport extends CommandBase {
  constructor() {
    super(import.meta.url);

    this.description('Reports on the binary size of the ffi as PR comments');

    this.argument('<target>', 'The os/arch target to report on');
  }

  override async run(target: string) {
    Log.title('Reporting on size changes');

    const prNumber = getPullRequestNumber();

    if (!prNumber) {
      Log.stepEnd('Not running on a PR', 'failure');
      return;
    }

    const octokit = await getOctokit();

    Log.stepBegin(`Fetching size info for ${target}`);
    const sizeInfo = await getFfiBinarySizes(target);
    Log.stepProgress(`File: ${sizeInfo.file}`);
    Log.stepProgress(`Bytes: ${sizeInfo.bytes}`);
    Log.stepEnd(`Size: ${sizeInfo.size}`);

    Log.stepBegin(`Fetching all previous size info`);
    const { result: allPreviousSizeComments, error } =
      await fetchPreviousSizeInfo(octokit);

    if (error || !allPreviousSizeComments) {
      Log.stepEnd('Failed to fetch previous size info', 'failure');
      throw error;
    }

    Object.values(allPreviousSizeComments).forEach((comment) => {
      Log.stepProgress(`${comment.path} | ${comment.size} | ${comment.bytes}`);
    });
    Log.stepEnd('Done fetching previous size info');

    const previousReportComment = await fetchPreviousReportComment(
      octokit,
      prNumber,
    );

    if (previousReportComment) {
      const content = updateReportContent(
        previousReportComment.body,
        target,
        sizeInfo,
        allPreviousSizeComments,
      );
      await updateComment(octokit, previousReportComment.id, target, content);
    } else {
      const content = createReportContent(
        target,
        sizeInfo,
        allPreviousSizeComments,
      );
      await createComment(octokit, target, content, prNumber);
    }

    Log.conclusion('Successfully reported on size changes');
  }
}

function updateReportContent(
  previousContent: string | undefined,
  target: string,
  sizeInfo: SizeInfo,
  allPreviousSizeComments: Record<string, SizeComment>,
) {
  if (!previousContent) {
    return createReportContent(target, sizeInfo, allPreviousSizeComments);
  }

  const comment = allPreviousSizeComments[`statsig-ffi-${target}`];

  const lines = previousContent.split('\n');

  return lines
    .map((line) => {
      if (line.includes(comment.path)) {
        return getLineForTarget(target, comment, sizeInfo);
      }
      return line;
    })
    .join('\n');
}

function createReportContent(
  target: string,
  sizeInfo: SizeInfo,
  allPreviousSizeComments: Record<string, SizeComment>,
) {
  const lines = [
    '## 📦 Size Report',
    '| File | Size | % Change |',
    '|--|--|--|',
  ];

  Object.entries(allPreviousSizeComments).forEach(([key, comment]) => {
    if (key === `statsig-ffi-${target}`) {
      lines.push(getLineForTarget(target, comment, sizeInfo));
    } else if (ONLY_REPORT_ON.includes(key)) {
      const size = covertToHumanReadableSize(comment.bytes, 'KB');

      lines.push(`| ${comment.path} | ${size} | - |`);
    }
  });

  return lines.join('\n');
}

function getLineForTarget(
  target: string,
  comment: SizeComment,
  sizeInfo: SizeInfo,
) {
  const beforeSize = covertToHumanReadableSize(comment.bytes, 'KB');
  const afterSize = covertToHumanReadableSize(sizeInfo.bytes, 'KB');

  const size = `${beforeSize} -> ${afterSize}`;
  const percent = ((sizeInfo.bytes - comment.bytes) / comment.bytes) * 100;

  let change = 'No Change';
  if (percent > 0) {
    change = '${\\color{orangered}⬆️}$ ' + percent.toFixed(2) + '%';
  } else if (percent < 0) {
    change = '${\\color{limegreen}⬇️}$ ' + percent.toFixed(2) + '%';
  }

  return `| statsig-ffi-${target} | ${size} | ${change} |`;
}

function getPullRequestNumber(): number | null {
  // refs/pull/160/merge
  const ref = process.env.GITHUB_REF;
  const result = ref?.replaceAll('/merge', '').split('/').pop();
  return result ? parseInt(result) : null;
}

async function fetchPreviousReportComment(octokit: Octokit, prNumber: number) {
  const { data } = await octokit.rest.issues.listComments({
    owner: 'statsig-io',
    repo: 'private-statsig-server-core',
    issue_number: prNumber,
    per_page: 100,
  });

  const found = data.find(
    (comment) =>
      comment.user?.login === 'statsig-kong[bot]' &&
      comment.body?.includes('## 📦 Size Report'),
  );

  return found;
}

async function createComment(
  octokit: Octokit,
  target: string,
  content: string,
  prNumber: number,
) {
  Log.stepBegin('Creating new size comment');

  const { data } = await octokit.rest.issues.createComment({
    owner: 'statsig-io',
    repo: 'private-statsig-server-core',
    issue_number: prNumber,
    body: content,
  });

  Log.stepEnd(`Successfully created comment [${data.id}] for ${target}`);
}

async function updateComment(
  octokit: Octokit,
  commentId: number,
  target: string,
  content: string,
) {
  Log.stepBegin(`Updating comment [${commentId}]`);

  await octokit.rest.issues.updateComment({
    owner: 'statsig-io',
    repo: 'private-statsig-server-core',
    comment_id: commentId,
    body: content,
  });

  Log.stepEnd(`Successfully updated comment [${commentId}] for ${target}`);
}
