import { Log } from "@/utils/terminal_utils.js";
import { CommandBase } from "./command_base.js";
import { getRootVersion } from "@/utils/toml_utils.js";
import { setOutput } from "@/utils/gh_utils.js";

export class GhGetLatest extends CommandBase {
  constructor() {
    super(import.meta.url);

    this.description('Output latest beta branch');
  }

  override async run(repository: string) {
    Log.title('Creating GitHub Release');

    const version = getRootVersion();
    const betaBranch = version.toBranch()
    setOutput('branch', betaBranch)
  }
}
