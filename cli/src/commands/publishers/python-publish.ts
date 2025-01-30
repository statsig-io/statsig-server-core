import { listFiles } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { execSync } from 'child_process';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

export async function publishPython(options: PublisherOptions) {
  const unzipped = listFiles(options.workingDir, 'statsig_python_core*');

  Log.stepBegin('Uploading to PyPI');

  unzipped.forEach((file) => {
    Log.stepBegin(`\nUploading ${path.basename(file)}`);
    const command = [
      'maturin upload',
      '--non-interactive',
      '--skip-existing',
      '--verbose',
      file,
    ].join(' ');

    Log.stepEnd(command);

    execSync(command, { cwd: options.workingDir, stdio: 'inherit' });
  });
}
