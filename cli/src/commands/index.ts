import { BumpVersion } from './bump_version.js';
import { GhAttachAssets } from './gh_attach_assets.js';
import { GhCreateRelease } from './gh_create_release.js';
import { GhPushPhp } from './gh_push_php.js';
import { JavaPub } from './java_pub.js';
import { NapiBuild } from './napi_build.js';
import { NapiPub } from './napi_pub.js';
import { PyBuild } from './py_build.js';
import { SizePersist } from './size_persist.js';
import { SizeReport } from './size_report.js';
import { SyncVersion } from './sync_version.js';
import { UnitTests } from './unit_tests.js';
import { ZipFiles } from './zip_files.js';

export const Commands = [
  new BumpVersion(),
  new GhAttachAssets(),
  new GhCreateRelease(),
  new GhPushPhp(),
  new JavaPub(),
  new NapiBuild(),
  new NapiPub(),
  new PyBuild(),
  new SizePersist(),
  new SizeReport(),
  new SyncVersion(),
  new UnitTests(),
  new ZipFiles(),
];
