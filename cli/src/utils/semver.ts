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

    const nonProd = version.split('-')[1];
    const parsedNonProd = nonProd ? nonProd.split('.') : null;
    this.rc = 0;
    this.beta = 0;
    if (parsedNonProd != null) {
      if (parsedNonProd[0] == 'rc') {
        this.rc = parseInt(parsedNonProd[1]);
      } else if (parsedNonProd[0] == 'beta') {
        this.beta = parseInt(parsedNonProd[1]);
      }
    }
  }

  toString(): string {
    let suffix = '';
    if (this.isRC()) {
      suffix = `-rc.${this.rc}`;
    } else if (this.isBeta()) {
      suffix = `-beta.${this.beta}`;
    }
    return `${this.major}.${this.minor}.${this.patch}${suffix}`;
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
