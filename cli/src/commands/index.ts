import { Build } from './build.js';
import { BumpVersion } from './bump-version.js';
import { GhAttachAssets } from './gh-attach-assets.js';
import { GhCreateRelease } from './gh-create-release.js';
import { GhPushPhp } from './gh-push-php.js';
import { JavaPub } from './java-pub.js';
import { NapiBuild } from './napi-build.js';
import { NapiPub } from './napi-pub.js';
import { Publish } from './publish.js';
import { PyBuild } from './py-build.js';
import { SizePersist } from './size-persist.js';
import { SizeReport } from './size-report.js';
import { SyncVersion } from './sync-version.js';
import { UnitTests } from './unit-tests.js';
import { ZipFiles } from './zip-files.js';

export const Commands = [
  new Build(),
  new BumpVersion(),
  new GhAttachAssets(),
  new GhCreateRelease(),
  new GhPushPhp(),
  new JavaPub(),
  new NapiBuild(),
  new NapiPub(),
  new Publish(),
  new PyBuild(),
  new SizePersist(),
  new SizeReport(),
  new SyncVersion(),
  new UnitTests(),
  new ZipFiles(),
];
