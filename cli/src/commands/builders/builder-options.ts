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
  subProject?: string;
  envSetupForBuild?: string; // Setup env variable to run build, e.g. RUSTFLAGS=""
  sign?: boolean;
};
