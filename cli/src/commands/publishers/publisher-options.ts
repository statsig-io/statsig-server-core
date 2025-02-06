export const PACKAGES = ['python', 'node', 'ffi', 'java', 'php'] as const;
export type Package = (typeof PACKAGES)[number];

export type PublisherOptions = {
  workflowId: string;
  package: Package;
  repository: string;
  workingDir: string;
  skipArtifactDownload: boolean;
  isProduction: boolean;
  disregardWorkflowChecks: boolean;
};
