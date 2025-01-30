export const PACKAGES = ['python', 'node', 'ffi'] as const;
export type Package = (typeof PACKAGES)[number];

export type PublisherOptions = {
  workflowId: string;
  package: Package;
  repository: string;
  workingDir: string;
  skipArtifactDownload: boolean;
};
