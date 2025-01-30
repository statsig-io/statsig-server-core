import { getFilenameWithoutExtension } from '@/utils/file_utils.js';
import { Argument, Command, Option } from 'commander';

export type OptionConfig = {
  flags: string;
  description: string;
  defaultValue?: unknown;
  choices?: readonly string[];
  required?: boolean;
};

function makeOption(config: OptionConfig) {
  const opt = new Option(config.flags, config.description);

  if (config.defaultValue) {
    opt.default(config.defaultValue);
  }

  if (config.choices) {
    opt.choices(config.choices);
  }

  if (config.required) {
    opt.makeOptionMandatory(true);
  }

  return opt;
}

export type ArgConfig = {
  name: string;
  description: string;
  required?: boolean;
  choices?: readonly string[];
};

function makeArg(config: ArgConfig) {
  const arg = new Argument(config.name, config.description);

  if (config.required) {
    arg.argRequired();
  }

  if (config.choices) {
    arg.choices(config.choices);
  }

  return arg;
}

type CommandConfig = {
  description?: string;
  options?: OptionConfig[];
  args?: ArgConfig[];
};

export abstract class CommandBase extends Command {
  constructor(metaUrl: string, config?: CommandConfig) {
    super(getFilenameWithoutExtension(metaUrl));

    if (config?.description) {
      this.description(config.description);
    }

    if (config?.options) {
      config.options.forEach((opt) => this.addOption(makeOption(opt)));
    }

    if (config?.args) {
      config.args.forEach((arg) => this.addArgument(makeArg(arg)));
    }

    this.action(this.run.bind(this));
  }

  protected abstract run(..._args: any[]): void;
}
