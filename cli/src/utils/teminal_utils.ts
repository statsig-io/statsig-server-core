import chalk from 'chalk';

export function printTitle(title: string) {
  console.log(chalk.bold(chalk.blue(`\n-------- ${title} --------\n`)));
}

export function printConclusion(conclusion: string) {
  console.log(chalk.bold(chalk.green(`\n-------- ${conclusion} --------\n`)));
}

export function printStepBegin(step: string) {
  console.log(chalk.bold(chalk.white(step)));
}

export function printStepProgress(step: string) {
  console.log(chalk.white(`├── ${step}`));
}

export function printStepEnd(step: string, kind?: 'success' | 'failure') {
  if (kind === 'success') {
    console.log(chalk.green(`└── ${step}\n`));
  } else if (kind === 'failure') {
    console.log(chalk.red(`└── ${step}\n`));
  } else {
    console.log(chalk.white(`└── ${step}\n`));
  }
}

export const Log = {
  title: printTitle,
  conclusion: printConclusion,
  stepBegin: printStepBegin,
  stepProgress: printStepProgress,
  stepEnd: printStepEnd,
  info: printStepBegin,
};
