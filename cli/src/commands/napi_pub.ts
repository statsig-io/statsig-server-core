import { BASE_DIR, ensureEmptyDir, getRootedPath } from '@/utils/file_utils.js';
import { parseTriple } from '@/utils/napi_utils.js';
import {
  downloadReleaseAsset,
  getAllAssetsForRelease,
  getOctokit,
  getReleaseByVersion,
} from '@/utils/octokit_utils.js';
import { Log } from '@/utils/teminal_utils.js';
import { getRootVersion } from '@/utils/toml_utils.js';
import AdmZip from 'adm-zip';
import { execSync } from 'child_process';
import { Command } from 'commander';
import { readFileSync, readdirSync, statSync, writeFileSync } from 'fs';
import { glob } from 'glob';

const TEMP_DIR = '/tmp/statsig-napi-build';

type Options = {
  production?: boolean;
};

export class NapiPub extends Command {
  constructor() {
    super('napi-pub');

    this.description('Publishes the statsig-napi package to NPM');

    this.argument(
      '<repo>',
      'The name of the repository, e.g. private-statsig-server-core',
    );

    this.option('--production', 'Whether to publish a production version');

    this.action(this.run.bind(this));
  }

  async run(repo: string, options: Options) {
    Log.title('Publishing statsig-napi to NPM');

    Log.stepBegin('Configuration');
    Log.stepEnd(`Repo: ${repo}`);

    const version = getRootVersion();
    const octokit = await getOctokit();

    ensureEmptyDir(TEMP_DIR);

    Log.stepBegin('Getting release');
    const release = await getReleaseByVersion(octokit, repo, version);
    if (!release) {
      Log.stepEnd('Release not found', 'failure');
      process.exit(1);
    }
    Log.stepEnd(`Release Found: ${release.html_url}`);

    Log.stepBegin('Getting all assets for release');
    const { assets, error } = await getAllAssetsForRelease(
      octokit,
      repo,
      release.id,
      'statsig-napi-',
    );

    if (error || !assets) {
      Log.stepEnd('Error getting assets', 'failure');
      console.error(
        error instanceof Error ? error.message : error ?? 'Unknown error',
      );
      process.exit(1);
    }

    if (assets.length === 0) {
      Log.stepEnd('No assets found', 'failure');
      process.exit(1);
    }

    assets.forEach((asset) => {
      Log.stepProgress(`${asset.name}`);
    });
    Log.stepEnd(`Found ${assets.length} assets`);

    Log.stepBegin('Downloading assets');
    const files = await Promise.all(
      assets.map(async (asset) => {
        const buffer = await downloadReleaseAsset(octokit, repo, asset.id);
        return { ...asset, buffer };
      }),
    );
    Log.stepEnd(`Downloaded ${files.length} files`);

    Log.stepBegin('Unzipping files');
    files.forEach((file) => {
      unzip(file.buffer);
      Log.stepProgress(`Completed: ${file.name}`);
    });
    Log.stepEnd('Unzipped files');

    Log.stepBegin('Aligning Npm Packages');
    (await moveNodeBinaries()).forEach(({ file, dir }) => {
      Log.stepProgress(`Moved: ${file} -> ${dir}`);
    });
    moveRootNapiPackage();
    Log.stepEnd('Aligned Npm Packages');

    Log.stepBegin('Syncing Napi Targets');
    const packageJson = syncNapiTargets();
    Log.stepProgress(
      `Targets: ${JSON.stringify(packageJson.optionalDependencies, null, 2)}`,
    );
    Log.stepEnd('Synced Napi Targets');

    Log.stepBegin('Publishing Npm Packages');
    await publishAllPackages(options.production === true);
    Log.stepEnd('Published Npm Packages');

    Log.conclusion('Successfully published statsig-napi to NPM');
  }
}

function unzip(buffer: ArrayBuffer) {
  const zip = new AdmZip(Buffer.from(buffer));

  zip.extractAllTo(TEMP_DIR, false, true);
}

function moveRootNapiPackage() {
  ensureEmptyDir(`${TEMP_DIR}/npm/statsig-napi`);
  execSync(`mv lib npm/statsig-napi/lib`, { cwd: TEMP_DIR });
  execSync(`mv package.json npm/statsig-napi`, { cwd: TEMP_DIR });
}

async function moveNodeBinaries() {
  execSync(`cp -r statsig-napi/npm ${TEMP_DIR}/npm`, { cwd: BASE_DIR });

  const nodeBinaries = await getNodeBinaries();
  const npmDirs = getNpmDirectories();

  const mapped: { file: string; dir: string }[] = [];
  nodeBinaries.forEach((file) => {
    const dir = npmDirs.findIndex((d) => file.includes(d));
    if (dir !== -1) {
      const dirName = npmDirs[dir];
      execSync(`mv ${file} npm/${dirName}`, { cwd: TEMP_DIR });

      npmDirs.splice(dir, 1);
      mapped.push({ file, dir: dirName });
    }
  });

  npmDirs.forEach((dir) => {
    execSync(`rm -rf npm/${dir}`, { cwd: TEMP_DIR });
  });

  return mapped;
}

async function publishAllPackages(isProduction: boolean) {
  const npmDirs = getNpmDirectories();

  npmDirs.forEach((dir) => {
    const packageJson = JSON.parse(
      readFileSync(`${TEMP_DIR}/npm/${dir}/package.json`, 'utf8'),
    );

    const err = publishPackage(dir, isProduction);
    if (err) {
      Log.stepEnd(`Failed to publish: ${packageJson.name}`, 'failure');
      console.error('Error: ', err.message);
      process.exit(1);
    } else {
      Log.stepProgress(
        `Published: ${packageJson.name} ${packageJson.version}`,
        'success',
      );
    }
  });
}

function getNpmDirectories() {
  const dirs = readdirSync(`${TEMP_DIR}/npm`).filter(
    (f) => f !== 'npm' && statSync(`${TEMP_DIR}/npm/${f}`).isDirectory(),
  );
  return dirs;
}

async function getNodeBinaries() {
  const files = await glob('**/*.node', {
    cwd: TEMP_DIR,
    ignore: 'node_modules/**',
  });
  return files;
}

function publishPackage(dir: string, isProduction: boolean): Error | null {
  const configPath = getRootedPath('.npmrc');
  const publish = [
    `npm publish`,
    `--registry=https://registry.npmjs.org/`,
    `--userconfig=${configPath}`,
    `--access public`,
    isProduction ? `` : '--tag beta',
  ];

  const command = publish.join(' ');
  try {
    execSync(command, { cwd: `${TEMP_DIR}/npm/${dir}` });
    return null;
  } catch (error) {
    return error as Error;
  }
}

function syncNapiTargets() {
  const packageJson = JSON.parse(
    readFileSync(`${TEMP_DIR}/npm/statsig-napi/package.json`, 'utf8'),
  );

  const optionalDependencies = { ...packageJson.optionalDependencies };

  const npmDirs = getNpmDirectories().reduce(
    (acc, dir) => {
      acc[dir] = `${TEMP_DIR}/npm/${dir}`;
      return acc;
    },
    {} as Record<string, string>,
  );

  const targets = packageJson.napi.targets as string[];

  targets.forEach((target) => {
    const nativeInfo = parseTriple(target);

    const dir = npmDirs[nativeInfo.platformArchABI];
    if (!dir) {
      throw new Error(`Target ${target} not found`);
    }

    const targetPackage = JSON.parse(
      readFileSync(`${dir}/package.json`, 'utf8'),
    );

    optionalDependencies[targetPackage.name] = targetPackage.version;
  });

  packageJson.optionalDependencies = optionalDependencies;
  writeFileSync(
    `${TEMP_DIR}/npm/statsig-napi/package.json`,
    JSON.stringify(packageJson, null, 2),
  );

  return packageJson;
}
