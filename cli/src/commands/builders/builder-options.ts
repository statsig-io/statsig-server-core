import { Distro, Platform } from '@/utils/docker_utils.js';

export type BuilderOptions = {
  release: boolean;
  platform: Platform;
  distro: Distro;
  outDir: string;
  skipDockerBuild: boolean;
};
