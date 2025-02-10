import { Log } from '@/utils/teminal_utils.js';

import { BuilderOptions } from './builder-options.js';
import { buildFfiHelper } from "@/utils/ffi_utils.js";

export function buildFfi(options: BuilderOptions) {
  Log.title(`Building statsig-ffi`);

  buildFfiHelper(options)

  Log.stepEnd(`Built statsig-ffi`);
}
