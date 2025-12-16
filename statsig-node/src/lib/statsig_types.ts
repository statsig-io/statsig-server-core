import { EvaluationDetails, SecondaryExposure } from './statsig-generated';

export type TypedGet = <T = unknown>(
  key: string,
  defaultValue: T,
  typeGuard?: ((value: unknown) => value is T | null) | null,
) => T;

export type UnknownGet = (
  key: string,
  fallback?: boolean | number | string | object | Array<any> | null,
) => unknown | null;

function _getTypeOf(x: unknown): string {
  return Array.isArray(x) ? 'array' : x === null ? 'null' : typeof x;
}

class BaseEvaluation {
  readonly name: string;
  readonly ruleID: string;
  readonly idType: string;
  readonly details: EvaluationDetails;
  readonly secondaryExposures: SecondaryExposure[] = [];

  constructor(name: string, data: Record<string, unknown>) {
    this.name = name;
    this.ruleID = (data.ruleID as string) ?? '';
    this.idType = (data.idType as string) ?? '';
    this.details = _extractEvaluationDetails(
      data.details as Record<string, unknown>,
    );
    this.secondaryExposures =
      (data.secondaryExposures as SecondaryExposure[]) ?? [];
  }

  getEvaluationDetails(): EvaluationDetails {
    return this.details;
  }

  getRuleId(): string {
    return this.ruleID;
  }

  getIdType(): string {
    return this.idType;
  }

  getSecondaryExposures(): SecondaryExposure[] {
    return this.secondaryExposures;
  }
}

export class FeatureGate extends BaseEvaluation {
  readonly value: boolean;

  constructor(name: string, raw: string | null) {
    const data = _parseRawEvaluation(raw);

    super(name, data);

    this.value = data.value === true;
  }
}

export class DynamicConfig extends BaseEvaluation {
  readonly value: Record<string, unknown> = {};

  readonly get: TypedGet;
  readonly getValue: UnknownGet;

  constructor(name: string, raw: string) {
    const data = _parseRawEvaluation(raw);

    super(name, data);

    this.value = (data.value as Record<string, unknown>) ?? {};

    this.get = _makeTypedGet(name, this.value);
    this.getValue = _makeUnknownGet(this.value);
  }
}

export class Experiment extends BaseEvaluation {
  readonly groupName: string | null = null;
  readonly value: Record<string, unknown> = {};

  readonly get: TypedGet;
  readonly getValue: UnknownGet;

  constructor(name: string, raw: string) {
    const data = _parseRawEvaluation(raw);

    super(name, data);

    this.groupName = (data.groupName as string) ?? null;
    this.value = (data.value as Record<string, unknown>) ?? {};

    this.get = _makeTypedGet(name, this.value);
    this.getValue = _makeUnknownGet(this.value);
  }

  getGroupName(): string | null {
    return this.groupName;
  }
}

export class Layer extends BaseEvaluation {
  readonly groupName: string | null = null;
  readonly allocatedExperimentName: string | null = null;
  readonly __value: Record<string, unknown> = {};

  readonly get: TypedGet;
  readonly getValue: UnknownGet;

  constructor(exposeFn: (param: string) => void, name: string, raw: string) {
    const data = _parseRawEvaluation(raw);

    super(name, data);

    this.__value = (data.value as Record<string, unknown>) ?? {};
    this.groupName = (data.groupName as string) ?? null;
    this.allocatedExperimentName =
      (data.allocatedExperimentName as string) ?? null;

    this.get = _makeTypedGet(name, this.__value, exposeFn);
    this.getValue = _makeUnknownGet(this.__value, exposeFn);
  }

  getGroupName(): string | null {
    return this.groupName;
  }

  getAllocatedExperimentName(): string | null {
    return this.allocatedExperimentName;
  }
}

function _parseRawEvaluation(raw: string | null): Record<string, unknown> {
  try {
    return JSON.parse(raw ?? '{}') as Record<string, unknown>;
  } catch (error) {
    console.error(`[Statsig] Error parsing BaseEvaluation: ${error}`);
    return {};
  }
}

function _makeTypedGet(
  name: string,
  value: Record<string, unknown>,
  exposeFunc?: (param: string) => void,
): TypedGet {
  return <T = unknown>(
    key: string,
    defaultValue: T,
    typeGuard: ((value: unknown) => value is T | null) | null = null,
  ): T => {
    // @ts-ignore - intentionally matches legacy behavior exactly
    defaultValue = defaultValue ?? null;

    // Equivalent to legacy `this.getValue(key, defaultValue)`
    const val = (value[key] ?? defaultValue) as unknown;

    if (val == null) {
      return defaultValue;
    }

    const expectedType = _getTypeOf(defaultValue);
    const actualType = _getTypeOf(val);

    if (typeGuard != null) {
      if (typeGuard(val)) {
        exposeFunc?.(key);
        return val as T;
      }

      console.warn(
        `[Statsig] Parameter type mismatch. '${name}.${key}' failed typeGuard. Expected '${expectedType}', got '${actualType}'`,
      );
      return defaultValue;
    }

    if (defaultValue == null || expectedType === actualType) {
      exposeFunc?.(key);
      return val as T;
    }

    console.warn(
      `[Statsig] Parameter type mismatch. '${name}.${key}' was found to be type '${actualType}' but fallback/return type is '${expectedType}'`,
    );
    return defaultValue;
  };
}

function _makeUnknownGet(
  value: Record<string, unknown>,
  exposeFunc?: (param: string) => void,
): UnknownGet {
  return (param: string, fallback?: unknown) => {
    if (fallback === undefined) {
      fallback = null;
    }

    if (param == null) {
      return fallback;
    }

    if (value[param] != null) {
      exposeFunc?.(param);
    }

    return value[param] ?? fallback;
  };
}

function _extractEvaluationDetails(
  data: Record<string, unknown> | null,
): EvaluationDetails {
  if (data == null) {
    return {
      reason: '',
      lcut: 0,
      receivedAt: 0,
      version: 0,
    };
  }
  return {
    reason: data.reason as string,
    lcut: data.lcut as number,
    receivedAt: data.received_at as number,
    version: data.version as number,
  };
}
