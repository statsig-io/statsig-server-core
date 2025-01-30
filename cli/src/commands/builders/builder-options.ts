import { Arch, OS } from '@/utils/docker_utils.js';

export type BuilderOptions = {
  release: boolean;
  arch: Arch;
  os: OS;
  outDir: string;
  skipDockerBuild: boolean;
  target?: string;
};
