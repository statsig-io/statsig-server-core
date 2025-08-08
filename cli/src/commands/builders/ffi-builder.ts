import { buildFfiHelper } from '@/utils/ffi_utils.js';
import { Log } from '@/utils/terminal_utils.js';

import { BuilderOptions } from './builder-options.js';

export function buildFfi(options: BuilderOptions) {
  Log.title(`Building statsig-ffi`);
  options.subProject = "statsig_ffi"
  buildFfiHelper(options)

  Log.stepEnd(`Built statsig-ffi`);
}
