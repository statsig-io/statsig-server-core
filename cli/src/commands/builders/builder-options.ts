import { Arch, OS } from '@/utils/docker_utils.js';

export type BuilderOptions = {
  targetProject?: string;
  release: boolean;
  arch: Arch;
  os: OS;
  outDir: string;
  skipDockerBuild: boolean;
  target?: string;
  docker: boolean;
};
