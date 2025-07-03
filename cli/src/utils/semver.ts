export class SemVer {
  major: number;
  minor: number;
  patch: number;
  beta: number;
  rc: number;

  constructor(version: string) {
    const [major, minor, patch] = version.split('.');

    this.major = parseInt(major);
    this.minor = parseInt(minor);
    this.patch = parseInt(patch);

    const beta = version.split('-')[1];
    this.beta = beta ? parseInt(beta.replace('beta.', '')) : 0;
  }

  toString(): string {
    return `${this.major}.${this.minor}.${this.patch}${
      this.isBeta() ? `-beta.${this.beta}` : ''
    }${this.isRC() ? `-rc.${this.rc}` : ''}`;
  }

  toBranch(): string {
    return this.beta
      ? `betas/${this.toString()}`
      : `releases/${this.toString()}`;
  }

  isBeta(): boolean {
    return this.beta > 0;
  }

  isRC(): boolean {
    return this.rc > 0;
  }
}
