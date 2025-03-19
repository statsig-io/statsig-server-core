import { glob } from 'glob';
import { Octokit } from 'octokit';

import {
  BASE_DIR,
  getFileSize,
  getHumanReadableSize,
  getRootedPath,
} from './file_utils.js';

export const REPORT_SIZE_ISSUE_NUMBER = 159;

export type SizeInfo = {
  file: string;
  path: string;
  bytes: number;
  size: string;
};

export type SizeComment = {
  path: string;
  bytes: number;
  size: string;
  commit: string;
  workflow_run_id: string;
};

export type SizeCommentWithId = SizeComment & {
  comment_id: number;
};

export async function getFfiBinarySizes(target: string): Promise<SizeInfo> {
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

    return { path, size, file, bytes };
  });

  const found = sizes.find((entry) => entry.file.includes(target));
  if (!found) {
    throw new Error(`No file found for target: ${target}`);
  }

  return found;
}

export async function fetchPreviousSizeInfo(octokit: Octokit): Promise<{
  result: Record<string, SizeCommentWithId> | null;
  error: Error | null;
}> {
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
        acc[json.path] = json;

        return acc;
      },
      {} as Record<string, SizeCommentWithId>,
    );

    return { result, error: null };
  } catch (e) {
    return { result: null, error: e as Error };
  }
}
