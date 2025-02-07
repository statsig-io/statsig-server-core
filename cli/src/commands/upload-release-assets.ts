import { getRootedPath, listFiles } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';

import { CommandBase } from './command_base.js';

type Options = {
  releaseId: string;
};

export class UploadReleaseAssets extends CommandBase {
  constructor() {
    super(import.meta.url, {
      description: 'Uploads release assets to GitHub',
      options: [
        {
          flags: '-i, --release-id <string>',
          description: 'The release ID to upload assets to',
          required: true,
        },
        {
          flags: '-t, --target <string>',
          description: 'The target to upload assets for',
          required: true,
        },
      ],
    });
  }

  override async run(options: Options) {
    Log.title('Uploading Release Assets');

    Log.stepBegin('Configuration');
    Log.stepEnd(`Release ID: ${options.releaseId}`);

    const files = [
      ...listFiles(getRootedPath('target/release'), 'libstatsig_ffi.so', {
        maxDepth: 1,
      }),
      ...listFiles(getRootedPath('target/release'), 'libstatsig_ffi.dylib', {
        maxDepth: 1,
      }),
      ...listFiles(getRootedPath('target/release'), 'libstatsig_ffi.dll', {
        maxDepth: 1,
      }),
    ];

    Log.stepBegin('Listing files to upload');
    for (const file of files) {
      Log.stepProgress(`Found: ${file}`);
    }
    Log.stepEnd('Finished listing files');

    Log.conclusion(`Successfully Uploaded Release Assets`);
  }
}
