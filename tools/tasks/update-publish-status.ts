import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';

type RuleStatus = {
  published: boolean;
};
type PackageStatus = {
  packageName: string;
  published: boolean;
  publishedVersion: string | null;
  rules: RuleStatus[];
};

const file = 'status.json';
const status = JSON.parse(readFileSync(file, 'utf8')) as PackageStatus[];

for (const plugin of status) {
  try {
    const version = execFileSync('pnpm', ['view', plugin.packageName, 'version'], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    }).trim();

    plugin.published = true;
    plugin.publishedVersion = version;
    for (const rule of plugin.rules) {
      rule.published = true;
    }
  } catch {
    plugin.published = false;
    plugin.publishedVersion = null;
    for (const rule of plugin.rules) {
      rule.published = false;
    }
  }
}

writeFileSync(file, `${JSON.stringify(status, null, 2)}\n`);
console.log(`Updated ${file}.`);
