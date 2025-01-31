import { listFiles } from '@/utils/file_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

export async function ffiPublish(options: PublisherOptions) {
  const binaries = [
    ...listFiles(options.workingDir, '*.a'),
    ...listFiles(options.workingDir, '*.dylib'),
    ...listFiles(options.workingDir, '*.so'),
    ...listFiles(options.workingDir, '*.dll'),
    ...listFiles(options.workingDir, '*.lib'),
  ];

  Log.stepBegin('Listing FFI Binaries');
  binaries.forEach((file) => {
    Log.stepProgress(`Found: ${path.basename(file)}`);
  });

  Log.stepEnd('Finished listing FFI Binaries');
}
