import {
  BASE_DIR,
  getFileSize,
  getHumanReadableSize,
  getRootedPath,
} from '@/utils/file_utils.js';
import { getOctokit } from '@/utils/octokit_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { Command } from 'commander';
import { glob } from 'glob';
import { Octokit } from 'octokit';

const REPORT_SIZE_ISSUE_NUMBER = 159;

type SizeInfo = {
  file: string;
  path: string;
  bytes: number;
  size: string;
};

type SizeComment = {
  name: string;
  bytes: number;
  size: string;
  commit: string;
  workflow_run_id: string;
};

type SizeCommentWithId = SizeComment & {
  comment_id: number;
};

export class ReportSize extends Command {
  constructor() {
    super('report-size');

    this.description('Comments on PRs with size changes');

    this.argument('<target>', 'The os/arch target to report on');

    this.action(this.run.bind(this));
  }

  async run(target: string) {
    Log.title('Reporting on size changes');

    const octokit = await getOctokit();

    const sizeInfo = await getFfiBinarySizes(target);
    const previousSizeInfo = await fetchPreviousSizeInfo(octokit);

    const found = previousSizeInfo[`statsig-ffi-${target}`];
    if (found && 'comment_id' in found) {
      await updateComment(octokit, found as SizeCommentWithId, sizeInfo);
    } else {
      await createComment(octokit, target, sizeInfo);
    }

    Log.conclusion('Successfully reported on size changes');
  }
}

async function getFfiBinarySizes(target: string): Promise<SizeInfo> {
  Log.stepBegin('Getting All Target Sizes');

  const allFiles = await glob(
    `target/**/{statsig_ffi,libstatsig_ffi}.{so,dll,dylib}`,
    {
      ignore: 'node_modules/**',
      cwd: BASE_DIR,
    },
  );

  const sizes: SizeInfo[] = allFiles.map((file) => {
    const path = getRootedPath(file);
    const bytes = getFileSize(path);
    const size = getHumanReadableSize(path);
    Log.stepProgress(`Found ${file}: ${size}`);
    return { path, size, file, bytes };
  });

  Log.stepEnd('Successfully got all target sizes');

  Log.stepBegin(`Getting ${target} sizes`);

  const found = sizes.find((entry) => entry.file.includes(target));
  if (!found) {
    Log.stepEnd(`No file found for target: ${target}`, 'failure');
    throw new Error(`No file found for target: ${target}`);
  }

  Log.stepProgress(`${found.file}: ${found.size}`);
  Log.stepEnd('Successfully got all sizes');

  return found;
}

async function fetchPreviousSizeInfo(
  octokit: Octokit,
): Promise<Record<string, SizeComment>> {
  Log.stepBegin('Fetching previous size infos');

  try {
    const { data } = await octokit.rest.issues.listComments({
      owner: 'statsig-io',
      repo: 'private-statsig-server-core',
      issue_number: REPORT_SIZE_ISSUE_NUMBER,
      per_page: 100,
    });

    const result = data.reduce(
      (acc, comment) => {
        const stripped = comment.body
          ?.replaceAll('```json', '')
          .replaceAll('```', '')
          .trim();
        const json = JSON.parse(stripped ?? '{}');
        json.comment_id = comment.id;
        acc[json.name] = json;

        Log.stepProgress(`Found [${json.comment_id}]: ${json.name}`);
        return acc;
      },
      {} as Record<string, SizeComment>,
    );

    Log.stepEnd('Successfully fetched previous size infos');
    return result;
  } catch (e) {
    Log.stepEnd('Failed to fetch previous size info', 'failure');
    throw e;
  }
}

async function createComment(
  octokit: Octokit,
  target: string,
  sizeInfo: SizeInfo,
) {
  Log.stepBegin('Creating new size comment');

  const comment: SizeComment = {
    name: `statsig-ffi-${target}`,
    bytes: sizeInfo.bytes,
    size: sizeInfo.size,
    commit: process.env.GITHUB_SHA ?? 'UNKNOWN',
    workflow_run_id: process.env.GITHUB_RUN_ID ?? 'UNKNOWN',
  };

  const { data } = await octokit.rest.issues.createComment({
    owner: 'statsig-io',
    repo: 'private-statsig-server-core',
    issue_number: REPORT_SIZE_ISSUE_NUMBER,
    body: `\`\`\`json\n${JSON.stringify(comment, null, 2)}\n\`\`\``,
  });

  Log.stepEnd(`Successfully created comment [${data.id}] for ${target}`);
}

async function updateComment(
  octokit: Octokit,
  comment: SizeCommentWithId,
  sizeInfo: SizeInfo,
) {
  Log.stepBegin(`Updating comment [${comment.comment_id}]`);

  const updatedComment: SizeCommentWithId = {
    comment_id: comment.comment_id,
    name: comment.name,
    bytes: sizeInfo.bytes,
    size: sizeInfo.size,
    commit: process.env.GITHUB_SHA ?? 'UNKNOWN',
    workflow_run_id: process.env.GITHUB_RUN_ID ?? 'UNKNOWN',
  };

  await octokit.rest.issues.updateComment({
    owner: 'statsig-io',
    repo: 'private-statsig-server-core',
    comment_id: comment.comment_id,
    body: `\`\`\`json\n${JSON.stringify(updatedComment, null, 2)}\n\`\`\``,
  });

  Log.stepEnd(`Successfully updated comment [${comment.comment_id}]`);
}
