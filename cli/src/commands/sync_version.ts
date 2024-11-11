import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import { Log, printTitle } from '@/utils/teminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import chalk from 'chalk';
import { Command } from 'commander';
import fs from 'fs';
import { glob } from 'glob';

export class SyncVersion extends Command {
  constructor() {
    super('sync-version');

    this.description('Sync the version across all relevant files');

    this.action(() => SyncVersion.sync());
  }

  static sync() {
    printTitle('Syncing Version');

    Log.stepBegin('Getting root version');
    const version = getRootVersion().toString();
    Log.stepEnd(`Root Version: ${version}`);

    updateStatsigMetadataVersion(version);
    updateNodePackageJsonVersions(version);
    updateJavaGradleVersion(version);
    updateStatsigGrpcDepVersion(version);
    Log.conclusion(`All Versions Updated to: ${version}`);
  }
}

function updateStatsigMetadataVersion(version: string) {
  Log.stepBegin('Updating statsig_metadata.rs');

  const path = getRootedPath('statsig-lib/src/statsig_metadata.rs');
  const contents = fs.readFileSync(path, 'utf8');

  const was = contents.match(/sdk_version: "([^"]+)"/)?.[1];
  const updated = contents.replace(
    /sdk_version: "([^"]+)"/,
    `sdk_version: "${version}"`,
  );

  fs.writeFileSync(path, updated, 'utf8');

  Log.stepEnd(`Updated Version: ${chalk.strikethrough(was)} -> ${version}`);
}

function updateNodePackageJsonVersions(version: string) {
  Log.stepBegin('Updating package.json');

  const paths = [getRootedPath('statsig-napi/package.json')];
  paths.push(
    ...glob.sync('statsig-napi/npm/**/package.json', {
      cwd: BASE_DIR,
      absolute: true,
    }),
  );

  paths.forEach((path) => {
    const contents = fs.readFileSync(path, 'utf8');
    const json = JSON.parse(contents);

    const was = contents.match(/version": "([^"]+)"/)?.[1];
    const updated = contents.replace(
      /version": "([^"]+)"/,
      `version": "${version}"`,
    );

    fs.writeFileSync(path, updated, 'utf8');

    Log.stepProgress(`${json.name}: ${chalk.strikethrough(was)} -> ${version}`);
  });

  Log.stepEnd('Updated all package.json files');
}

function updateJavaGradleVersion(version: string) {
  Log.stepBegin('Updating gradle.properties');

  const path = getRootedPath('statsig-ffi/bindings/java/gradle.properties');
  const contents = fs.readFileSync(path, 'utf8');

  const was = contents.match(/version=([^"]+)/)?.[1];
  const updated = contents.replace(/version=([^"]+)/, `version=${version}`);

  fs.writeFileSync(path, updated, 'utf8');

  Log.stepEnd(`Updated Version: ${chalk.strikethrough(was)} -> ${version}`);
}

function updateStatsigGrpcDepVersion(version: string) {
  Log.stepBegin('Updating statsig-lib -> statsig-grpc dependency version');

  const path = getRootedPath('statsig-lib/Cargo.toml');
  const contents = fs.readFileSync(path, 'utf8');

  const was = contents.match(/sigstat-grpc = \{[^}]*version = "([^"]+)"/)?.[1];
  const updated = contents.replace(
    /(sigstat-grpc = \{[^}]*version = )"([^"]+)"/,
    `$1"${version}"`,
  );

  fs.writeFileSync(path, updated, 'utf8');

  Log.stepEnd(`Updated Version: ${chalk.strikethrough(was)} -> ${version}`);
}
