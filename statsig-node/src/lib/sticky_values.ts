import { SecondaryExposure } from './statsig-generated';

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
