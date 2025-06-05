import { listFiles } from '@/utils/file_utils.js';
import { Log } from '@/utils/terminal_utils.js';
import { execSync } from 'child_process';
import path from 'node:path';

import { PublisherOptions } from './publisher-options.js';

const REQUIRED_FILES = [
  'cp37-abi3-macosx_10_12_x86_64.whl',
  'cp37-abi3-macosx_11_0_arm64.whl',
  'cp37-abi3-manylinux_2_17_aarch64.manylinux2014_aarch64.whl',
  'cp37-abi3-manylinux_2_17_x86_64.manylinux2014_x86_64.whl',
  'cp37-abi3-manylinux_2_27_aarch64.whl',
  'cp37-abi3-manylinux_2_27_x86_64.whl',
  'cp37-abi3-manylinux_2_28_aarch64.whl',
  'cp37-abi3-manylinux_2_28_x86_64.whl',
  'cp37-abi3-manylinux_2_34_aarch64.whl',
  'cp37-abi3-manylinux_2_34_x86_64.whl',
  'cp37-abi3-musllinux_1_2_aarch64.whl',
  'cp37-abi3-musllinux_1_2_x86_64.whl',
  'cp37-abi3-win_amd64.whl',
  'cp37-abi3-win32.whl',
];

export async function publishPython(options: PublisherOptions) {
  Log.stepBegin('Uploading to PyPI');

  const { maturinVersion, pythonVersion, pipVersion } = getToolInfo();
  Log.stepProgress(`Maturin: ${maturinVersion}`);
  Log.stepProgress(`Python: ${pythonVersion}`);
  Log.stepEnd(`Pip: ${pipVersion}`);

  Log.stepBegin('Listing files to upload');

  const wheels: Record<string, string> = {};
  listFiles(options.workingDir, 'statsig_python_core*.whl').forEach((file) => {
    const basename = path.basename(file);
    wheels[basename] = file;
  });

  const toFind = [...REQUIRED_FILES];
  let allValid = true;

  Object.entries(wheels).forEach(([basename, file]) => {
    const found = toFind.findIndex((f) => basename.includes(f));
    if (found === -1) {
      Log.stepProgress(`File not expected: ${basename}`, 'failure');
      allValid = false;
    } else {
      Log.stepProgress(`Found file ${basename}: ${file}`);
      toFind.splice(found, 1);
    }
  });

  if (!allValid) {
    Log.stepEnd('Some files were not expected', 'failure');
    return;
  }

  if (toFind.length > 0) {
    Log.stepEnd(
      `Not all files were found. Missing: ${toFind.join(', ')}`,
      'failure',
    );
    return;
  }

  Log.stepEnd('Finished listing files');

  const command = [
    'maturin upload',
    '--non-interactive',
    '--skip-existing',
    '--verbose',
    ...Object.values(wheels).map((file) => `${file}`),
  ].join(' ');

  Log.stepBegin('Uploading Wheels to PyPI');
  try {
    execSync(command, { cwd: options.workingDir, stdio: 'inherit' });
    Log.stepEnd('Successfully uploaded wheels to PyPI');
  } catch (e) {
    console.error(e);
    process.exit(1);
  }

  const source = listFiles(
    options.workingDir,
    'statsig_python_core*.tar.gz',
  ).pop();

  if (!source) {
    Log.stepEnd('No source distribution found', 'failure');
    return;
  }

  Log.stepBegin('Uploading Source Distribution to PyPI');
  Log.stepProgress(source);
  const sourceCommand = [
    'maturin upload',
    '--non-interactive',
    '--skip-existing',
    '--verbose',
    source,
  ].join(' ');

  execSync(sourceCommand, { cwd: options.workingDir, stdio: 'inherit' });
  Log.stepEnd('Successfully uploaded source distribution to PyPI');
}

function getToolInfo() {
  const maturinVersion = execSync('maturin --version').toString().trim();
  const pythonVersion = execSync('python3 --version').toString().trim();
  const pipVersion = execSync('pip3 --version').toString().trim();
  return {
    maturinVersion,
    pythonVersion,
    pipVersion,
  };
}
