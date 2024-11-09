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
import { basename } from 'path';

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
    const paths = getStoragePaths(files);
    if (paths.length === 0) {
      Log.stepEnd('No files found matching pattern', 'failure');
      process.exit(1);
    }
    files.forEach((file) => {
      Log.stepProgress(`Found: ${file}`);
    });

    Log.stepEnd(`Files Found: ${paths.length}`);

    Log.stepBegin('Creating zip file');
    Log.stepProgress(`Output: ${options.output}`);

    const zip = new AdmZip();

    for (const { filePath, storagePath } of paths) {
      const fullPath = getRootedPath(filePath);
      const contents = readFileSync(fullPath);

      zip.addFile(storagePath, contents);
      Log.stepProgress(`Added File: ${storagePath}`);
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

function getStoragePaths(paths: string[]) {
  if (paths.length === 0) {
    return [];
  }

  if (paths.length === 1) {
    return [{ filePath: paths[0], storagePath: basename(paths[0]) }];
  }

  const splitPaths = paths.map((path) => path.split('/').filter(Boolean));

  const commonPrefix = [];
  for (let i = 0; i < splitPaths[0].length; i++) {
    const component = splitPaths[0][i];
    if (splitPaths.every((path) => path[i] === component)) {
      commonPrefix.push(component);
    } else {
      break;
    }
  }

  return paths.map((path) => {
    const strippedPath = path.split('/').slice(commonPrefix.length).join('/');
    return {
      filePath: path,
      storagePath: `/${strippedPath}`,
    };
  });
}
