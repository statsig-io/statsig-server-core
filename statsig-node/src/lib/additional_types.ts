import { SecondaryExposure, AmplitudeEvent } from './statsig-generated';

export type StickyValues = {
  value: boolean;
  json_value: Record<string, unknown>;
  rule_id: string;
  group_name: string | null;
  secondary_exposures: SecondaryExposure[];
  undelegated_secondary_exposures: SecondaryExposure[];
  config_delegate: string | null;
  explicit_parameters: string[] | null;
  time: number;
  configVersion?: number | undefined;
};

export type UserPersistedValues = Record<string, StickyValues>;

export type SdkEvent =
  | '*'
  | 'gate_evaluated'
  | 'dynamic_config_evaluated'
  | 'experiment_evaluated'
  | 'layer_evaluated'
  | 'specs_updated';


export type AmplitudeMiddlewarePayload = {
  event: AmplitudeEvent;
  extra: unknown;
};

export type AmplitudeCallbacks =
  | ((event: AmplitudeEvent, status: number, message: string) => void)
  | { onLogEventServerResponse(event: AmplitudeEvent, status: number, message: string): void };

export type AmplitudeMiddleware = (
  payload: AmplitudeMiddlewarePayload,
  next: (payload?: AmplitudeMiddlewarePayload) => Promise<AmplitudeMiddlewarePayload | null>,
) => unknown | Promise<unknown>;
