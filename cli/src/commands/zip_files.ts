import {
  BASE_DIR,
  getHumanReadableSize,
  getRootedPath,
} from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import AdmZip from 'adm-zip';
import { Command } from 'commander';
import { readFileSync } from 'fs';
import { glob } from 'glob';

type Options = {
  output: string;
};

export class ZipFiles extends Command {
  constructor() {
    super('zip-files');

    this.description('Bump the version of the SDK');

    this.requiredOption(
      '--output, <string>',
      'File path of resulting zip file',
    );

    this.argument('<pattern>', 'Glob pattern matching files to zip');
    this.action(this.run.bind(this));
  }

  async run(pattern: string, options: Options) {
    Log.title('Zipping Files');

    Log.stepBegin('Configuration');
    Log.stepProgress(`Pattern: ${pattern}`);
    Log.stepEnd(`Output: ${options.output}`);

    Log.stepBegin('Finding files');
    const files = await getFilesMatchingPattern(pattern);
    Log.stepEnd(`Files Found: ${files.length}`);

    Log.stepBegin('Creating zip file');
    Log.stepProgress(`Output: ${options.output}`);

    const zip = new AdmZip();

    for (const file of files) {
      const fullPath = getRootedPath(file);
      const contents = readFileSync(fullPath);

      const parts = file.split('/');
      const storageName = parts.length > 2 ? parts.slice(-2).join('/') : file;

      zip.addFile(storageName, contents);
      Log.stepProgress(`Added File: ${storageName}`);
    }

    const zipFilename = options.output.endsWith('.zip')
      ? options.output
      : `${options.output}.zip`;

    const fullPath = getRootedPath(zipFilename);
    zip.writeZip(fullPath);

    const size = getHumanReadableSize(fullPath);
    Log.stepProgress(`Size: ${size}`);
    Log.stepEnd(`Path: ${fullPath}`);

    console.log('✅ Successfully created zip file');
  }
}

async function getFilesMatchingPattern(pattern: string) {
  const files = await glob(pattern, {
    ignore: 'node_modules/**',
    cwd: BASE_DIR,
  });
  return files;
}
