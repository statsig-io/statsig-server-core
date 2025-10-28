export const PACKAGES = [
  'python',
  'node',
  'ffi',
  'java',
  'php',
  'dotnet',
  'elixir',
  'go',
  'cpp',
] as const;
export type Package = (typeof PACKAGES)[number];

export type PublisherOptions = {
  workflowId: string;
  package: Package;
  repository: string;
  releaseId: number;
  workingDir: string;
  skipArtifactDownload: boolean;
  disregardWorkflowChecks: boolean;
};
