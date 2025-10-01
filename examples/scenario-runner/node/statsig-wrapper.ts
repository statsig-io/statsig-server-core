import { Statsig, StatsigUser } from '@statsig/statsig-node-core';
import StatsigLegacy, { StatsigUser as StatsigLegacyUser } from 'statsig-node';

const SCRAPI_URL = 'http://scrapi:8000';

export class StatsigWrapper {
  static isCore = false;

  private static statsig: Statsig;
  private static user: StatsigUser | StatsigLegacyUser | null = null;

  static async initialize() {
    const variant = process.env.SDK_VARIANT!;

    if (variant === 'core') {
      this.isCore = true;
      this.statsig = new Statsig('secret-NODE_CORE', {
        specsUrl: `${SCRAPI_URL}/v2/download_config_specs`,
        logEventUrl: `${SCRAPI_URL}/v1/log_event`,
        // disableCountryLookup: true,
      });

      await this.statsig.initialize();
      return;
    }

    if (variant === 'legacy') {
      this.isCore = false;

      await StatsigLegacy.initialize('secret-NODE_LEGACY', {
        api: `${SCRAPI_URL}/v1`,
      });
      return;
    }

    throw new Error(`Invalid SDK variant: ${variant}`);
  }

  static setUser(userData: Record<string, unknown>) {
    if (this.isCore) {
      const user = StatsigUser.withUserID(userData.userID as string);
      this.user = user;
    } else {
      this.user = userData as StatsigLegacyUser;
    }
  }

  static checkGate(gateName: string) {
    if (this.isCore) {
      this.validateCoreUser();

      return this.statsig.checkGate(this.user as StatsigUser, gateName);
    }

    return StatsigLegacy.checkGateSync(
      this.user as StatsigLegacyUser,
      gateName,
    );
  }

  static logEvent(eventName: string) {
    if (this.isCore) {
      this.validateCoreUser();
      this.statsig.logEvent(this.user as StatsigUser, eventName);
    } else {
      StatsigLegacy.logEvent(this.user as StatsigLegacyUser, eventName);
    }
  }

  static getClientInitResponse() {
    if (this.isCore) {
      this.validateCoreUser();
      return this.statsig.getClientInitializeResponse(this.user as StatsigUser);
    }

    return StatsigLegacy.getClientInitializeResponse(
      this.user as StatsigLegacyUser,
    );
  }

  private static validateCoreUser() {
    if (!(this.user instanceof StatsigUser)) {
      throw new Error('User not set or not a StatsigUser');
    }
  }
}
