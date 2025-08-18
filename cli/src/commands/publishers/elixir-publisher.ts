import { BASE_DIR, getRootedPath } from '@/utils/file_utils.js';
import {
  GhRelease,
  createReleaseForVersion,
  getOctokit,
  uploadReleaseAsset,
} from '@/utils/octokit_utils.js';
import { SemVer } from '@/utils/semver.js';
import { Log } from '@/utils/terminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import { execSync } from 'child_process';
import fs from 'fs';
import { glob } from 'glob';
import { Octokit } from 'octokit';
import path from 'path';

import { PublisherOptions } from './publisher-options.js';

const ELIXIR_REPRO_NAME = 'statsig-elixir-core';
const EXPECTED_ZIPPED_FILES = 6;
const COMPRESSED_DIR = 'artifacts/elixir_compressed_dir';
const ELIXIR_DIR = `${BASE_DIR}/statsig-elixir`;

export async function publishElixir(options: PublisherOptions) {
  const octokit = await getOctokit();
  const version = getRootVersion();

  // step 1. create release
  const release = await createRelease(octokit, version);
  // step 2. zip path is **/target/**/release/libstatsig_elixir**.so
  const zippedFilesPath = await compressLibraries();
  // step 3. upload
  for (const path of zippedFilesPath) {
    await uploadRelease(octokit, release, path);
  }
  // step 4. run checksum
  await runCheckSum();
  // step 5. publish
  await publishToHex();
}

async function publishToHex() {
  Log.stepBegin('Publish package to hex');
  execSync(`HEX_API_KEY=${process.env.HEX_API_KEY} mix hex.publish --yes`, { cwd: ELIXIR_DIR, stdio: 'inherit' });
}

async function runCheckSum() {
  Log.stepBegin('Setup elixir build environment');
  execSync('mix local.hex', { cwd: ELIXIR_DIR, stdio: 'inherit'});
  execSync('mix local.rebar', { cwd: ELIXIR_DIR, stdio: 'inherit'});
  execSync('mix deps.get', { cwd: ELIXIR_DIR, stdio: 'inherit'});
  Log.stepEnd('Setup elixir build environment');

  Log.stepBegin('Rerun checksum');
  execSync('FORCE_STATSIG_NATIVE_BUILD="true" mix compile', {
    cwd: ELIXIR_DIR,
    stdio: 'inherit'
  });
  execSync(`mix rustler_precompiled.download Statsig.NativeBindings --all --printls`, {
    cwd: ELIXIR_DIR,
    stdio: 'inherit'
  });
  Log.stepEnd('Rerun checksum');
}

async function uploadRelease(
  octokit: Octokit,
  release: GhRelease,
  path: string,
) {
  const uploadUrl = release.upload_url;
  if (!uploadUrl) {
    Log.stepEnd('No upload URL found', 'failure');
    process.exit(1);
  }

  Log.stepProgress(`Release upload URL: ${uploadUrl}`);
  const { result, error } = await uploadReleaseAsset(
    octokit,
    ELIXIR_REPRO_NAME,
    release.id,
    path,
  );
}

async function createRelease(octokit: Octokit, version: SemVer) {
  Log.stepBegin('Creating release');

  const { result: newRelease, error } = await createReleaseForVersion(
    octokit,
    ELIXIR_REPRO_NAME,
    version,
  );

  if (!newRelease) {
    Log.stepEnd(`Failed to create release`, 'failure');
    console.error(error ?? 'Unknown error');
    process.exit(1);
  }

  Log.stepEnd(`Release created ${newRelease.html_url}`);

  return newRelease;
}

async function compressLibraries() {
  const compressedPath = [];
  Log.stepBegin('Compressing: Create tar gz files');
  const matches = await glob(
    '/tmp/statsig-server-core-publish/**/target/**/release/libstatsig_elixir**.so',
    {
      nodir: true,
    },
  );

  if (matches.length < EXPECTED_ZIPPED_FILES) {
    console.error('Found less binaries');

    process.exit(1);
  }

  fs.mkdirSync(COMPRESSED_DIR, { recursive: true });

  for (const filePath of matches) {
    const dir = path.dirname(filePath);
    const baseName = path.basename(filePath);
    const tarName = path.join(COMPRESSED_DIR, `${baseName}.tar.gz`);

    console.log(`Compressing: ${filePath} -> ${tarName}`);
    execSync(`tar -czf ${tarName} -C "${dir}" "${baseName}"`, {
      stdio: 'inherit',
    });
    compressedPath.push(tarName);
  }
  Log.stepEnd('Compressing: Create tar gz files');
  if (compressedPath.length <= EXPECTED_ZIPPED_FILES) {
    console.error('Found less zipped files');
    process.exit(1);
  }
  return compressedPath;
}
