import { getOctokit } from '@/utils/octokit_utils.js';
import {
  REPORT_SIZE_ISSUE_NUMBER,
  SizeComment,
  SizeCommentWithId,
  SizeInfo,
  fetchPreviousSizeInfo,
  getFfiBinarySizes,
} from '@/utils/size_report_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { Command } from 'commander';
import { Octokit } from 'octokit';

export class SizePersist extends Command {
  constructor() {
    super('size-persist');

    this.description('Keeps track of the binary size of the ffi');

    this.argument('<target>', 'The os/arch target to report on');

    this.action(this.run.bind(this));
  }

  async run(target: string) {
    Log.title('Reporting on size changes');

    if (!isRunningOnMain()) {
      Log.stepEnd('Not running on main', 'failure');
      return;
    }

    const octokit = await getOctokit();

    const sizeInfo = await getFfiBinarySizes(target);
    const { result: allPreviousSizeComments, error } =
      await fetchPreviousSizeInfo(octokit);

    if (error || !allPreviousSizeComments) {
      Log.stepEnd('Failed to fetch previous size info', 'failure');
      throw error;
    }

    const previousSizeComment =
      allPreviousSizeComments[`statsig-ffi-${target}`];

    if (previousSizeComment && 'comment_id' in previousSizeComment) {
      await updateComment(
        octokit,
        previousSizeComment as SizeCommentWithId,
        sizeInfo,
      );
    } else {
      await createComment(octokit, target, sizeInfo);
    }

    Log.conclusion('Successfully reported on size changes');
  }
}

function isRunningOnMain() {
  const ref = process.env.GITHUB_REF;
  return ref === 'refs/heads/main';
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
