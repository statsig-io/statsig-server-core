import AdmZip from 'adm-zip';
import { existsSync, mkdirSync, rmSync, statSync } from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url); // get the resolved path to the file
const __dirname = path.dirname(__filename); // get the name of the directory

export const BASE_DIR = path.resolve(__dirname, '..', '..', '..');

export function getRootedPath(filepath: string) {
  return path.resolve(BASE_DIR, filepath);
}

export function getFileSize(filepath: string) {
  const stats = statSync(filepath);
  return stats.size;
}

export function getHumanReadableSize(
  filepath: string,
  maxUnit: 'B' | 'KB' | 'MB' | 'GB' = 'GB',
) {
  const bytes = getFileSize(filepath);

  return covertToHumanReadableSize(bytes, maxUnit);
}

export function covertToHumanReadableSize(
  bytes: number,
  maxUnit: 'B' | 'KB' | 'MB' | 'GB' = 'GB',
) {
  if (bytes < 1024 || maxUnit === 'B') {
    return `${bytes} Bytes`;
  }

  if (bytes < 1024 * 1024 || maxUnit === 'KB') {
    return `${(bytes / 1024).toFixed(0)} KB`;
  }

  if (bytes < 1024 * 1024 * 1024 || maxUnit === 'MB') {
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }

  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

export function ensureEmptyDir(dir: string) {
  if (existsSync(dir)) {
    rmSync(dir, { recursive: true, force: true });
  }

  mkdirSync(dir, { recursive: true });
}

export function unzip(buffer: ArrayBuffer, targetDir: string) {
  const zip = new AdmZip(Buffer.from(buffer));

  zip.extractAllTo(targetDir, false, true);
}
