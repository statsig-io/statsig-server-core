import { Arch, Distro } from '@/utils/docker_utils.js';

export type BuilderOptions = {
  release: boolean;
  arch: Arch;
  distro: Distro;
  outDir: string;
  skipDockerBuild: boolean;
};
