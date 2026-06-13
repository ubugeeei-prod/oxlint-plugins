import { spawnSync } from 'node:child_process';

type AuditFinding = {
  paths?: string[];
};
type Advisory = {
  title?: string;
  module_name?: string;
  severity?: string;
  url?: string;
  findings?: AuditFinding[];
};
type AuditJson = {
  advisories?: Record<string, Advisory>;
};

const publishablePrefixes = [
  '@oxlint-plugins/oxlint-plugin-type-aware>',
  '@oxlint-plugins/oxlint-plugin-no-forbidden-identifiers>',
  '@oxlint-plugins/oxlint-plugin-e18e>',
  '@oxlint-plugins/oxlint-plugin-eslint-comments>',
  '@oxlint-plugins/oxlint-plugin-eslint-json>',
  '@oxlint-plugins/oxlint-plugin-react-refresh>',
  '@oxlint-plugins/oxlint-plugin-security>',
  '@oxlint-plugins/oxlint-plugin-cypress>',
  '@oxlint-plugins/oxlint-plugin-mocha>',
  '@oxlint-plugins/oxlint-plugin-simple-import-sort>',
  '@oxlint-plugins/oxlint-plugin-unused-imports>',
  '@oxlint-plugins/oxlint-plugin-storybook>',
  '@oxlint-plugins/oxlint-plugin-stylistic>',
];

const audit = spawnSync('pnpm', ['audit', '--prod', '--no-optional', '--json'], {
  encoding: 'utf8',
  stdio: ['ignore', 'pipe', 'pipe'],
});

const output = audit.stdout.trim();
if (!output) {
  if (audit.status === 0) {
    console.log('No production advisories found.');
    process.exit(0);
  }

  process.stderr.write(audit.stderr);
  process.exit(audit.status ?? 1);
}

const report = JSON.parse(output) as AuditJson;
const failures: string[] = [];

for (const advisory of Object.values(report.advisories ?? {})) {
  for (const finding of advisory.findings ?? []) {
    for (const path of finding.paths ?? []) {
      if (!publishablePrefixes.some((prefix) => path.startsWith(prefix))) {
        continue;
      }

      failures.push(
        `${advisory.severity ?? 'unknown'} ${advisory.module_name ?? 'unknown'}: ${advisory.title ?? 'untitled'} (${path}) ${advisory.url ?? ''}`.trim(),
      );
    }
  }
}

if (failures.length > 0) {
  console.error('Production audit failures for publishable packages:');
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log('Publishable package production dependencies have no blocking advisories.');
