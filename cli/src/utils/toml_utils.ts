import fs from 'fs';
import { parse } from 'smol-toml';

import { getRootedPath } from './file_utils.js';
import { SemVer } from './semver.js';

type RootToml = {
  workspace: { package: { version: string } };
};

export function getRootVersion(): SemVer {
  const tomlPath = getRootedPath('Cargo.toml');
  const tomlData = fs.readFileSync(tomlPath, 'utf8');
  const toml = parse(tomlData) as RootToml;

  return new SemVer(toml['workspace']['package']['version']);
}

export function setRootVersion(version: SemVer) {
  const tomlPath = getRootedPath('Cargo.toml');
  const tomlData = fs.readFileSync(tomlPath, 'utf8');
  const versionRegex = /version\s*=\s*"[^"]*"/;
  const updatedToml = tomlData.replace(
    versionRegex,
    `version = "${version.toString()}"`,
  );

  fs.writeFileSync(tomlPath, updatedToml);
}
