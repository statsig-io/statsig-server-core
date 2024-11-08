import { statSync } from 'fs';
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

export function getHumanReadableSize(filepath: string) {
  const bytes = getFileSize(filepath);

  if (bytes < 1024) {
    return `${bytes} Bytes`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(2)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}
