import { BumpVersion } from './bump_version.js';
import { GhAttachAssets } from './gh_attach_assets.js';
import { GhCreateRelease } from './gh_create_release.js';
import { GhPushPhp } from './gh_push_php.js';
import { JavaPub } from './java_pub.js';
import { NapiBuild } from './napi_build.js';
import { NapiPub } from './napi_pub.js';
import { ReportSize } from './report_size.js';
import { SyncVersion } from './sync_version.js';
import { ZipFiles } from './zip_files.js';

export const Commands = [
  new BumpVersion(),
  new GhAttachAssets(),
  new GhCreateRelease(),
  new GhPushPhp(),
  new JavaPub(),
  new NapiBuild(),
  new NapiPub(),
  new ReportSize(),
  new SyncVersion(),
  new ZipFiles(),
];
