import { listFiles } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

export async function publishPython(options: PublisherOptions) {
  Log.stepBegin('Uploading to PyPI');

  const { maturinVersion, pythonVersion, pipVersion } = getToolInfo();
  Log.stepProgress(`Maturin: ${maturinVersion}`);
  Log.stepProgress(`Python: ${pythonVersion}`);
  Log.stepEnd(`Pip: ${pipVersion}`);

  Log.stepBegin('Listing files to upload');
  let hasSeenSource = false;
  const unzipped = listFiles(options.workingDir, 'statsig_python_core*').filter(
    (file) => {
      if (file.endsWith('.tar.gz')) {
        if (hasSeenSource) {
          return false;
        }

        hasSeenSource = true;
        return true;
      }

      return true;
    },
  );
  unzipped.forEach((file) => {
    Log.stepProgress(`Found file ${file}`);
  });

  Log.stepEnd('Finished listing files');

  let allFilesUploaded = true;

  const seen: Record<string, string> = {};

  unzipped.forEach((file) => {
    const filename = path.basename(file);
    const command = [
      'maturin upload',
      '--non-interactive',
      '--skip-existing',
      '--verbose',
      file,
    ].join(' ');

    Log.stepBegin(`\nUploading ${filename}`);

    if (seen[filename]) {
      allFilesUploaded = false;
      Log.stepEnd(
        `${filename} already uploaded from ${seen[filename]}`,
        'failure',
      );
      return;
    }

    seen[filename] = file;

    Log.stepProgress(command);

    try {
      execSync(command, { cwd: options.workingDir, stdio: 'inherit' });
      Log.stepEnd(`Uploaded ${filename}`, 'success');
    } catch (e) {
      Log.stepEnd(`Failed to upload ${path.basename(file)}`, 'failure');
      console.error(e);
      allFilesUploaded = false;
    }
  });

  if (!allFilesUploaded) {
    throw new Error('Failed to upload all files');
  }
}

function getToolInfo() {
  const maturinVersion = execSync('maturin --version').toString().trim();
  const pythonVersion = execSync('python3 --version').toString().trim();
  const pipVersion = execSync('pip3 --version').toString().trim();
  return {
    maturinVersion,
    pythonVersion,
    pipVersion,
  };
}
