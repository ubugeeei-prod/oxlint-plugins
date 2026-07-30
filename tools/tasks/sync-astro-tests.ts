// Copies the authored eslint-plugin-astro fixtures for the implemented slice
// from the pinned upstream submodule into one deterministic replay fixture.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { basename, join, relative } from 'node:path';

type Manifest = {
  plugins: Array<{
    id: string;
    npm: string;
    submodule: string;
    baselineVersion: string;
    pinnedRef?: string;
    license: string;
  }>;
};

type AuthoredError = {
  message: string;
  line: number;
  column: number;
};

type FixtureCase = {
  filename: string;
  code: string;
  errors?: AuthoredError[];
  output?: string;
};

const ROOT = process.cwd();
const manifest = JSON.parse(
  readFileSync(join(ROOT, 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;
const plugin = manifest.plugins.find((entry) => entry.id === 'eslint-plugin-astro');
if (!plugin) {
  throw new Error('eslint-plugin-astro is not registered in tools/port-targets.json');
}

const expectedVersion = '3.0.1';
const expectedRef = 'v3.0.1';
const expectedCommit = 'd887cce6ad7d2cacfde885b29acfe080a7be6a85';
if (plugin.baselineVersion !== expectedVersion || plugin.pinnedRef !== expectedRef) {
  throw new Error(
    `Astro fixture sync expects ${expectedRef}; port target is ${plugin.pinnedRef ?? plugin.baselineVersion}.`,
  );
}

const upstreamRoot = join(ROOT, plugin.submodule);
const fixtureRoot = join(upstreamRoot, 'tests', 'fixtures', 'rules');
if (!existsSync(fixtureRoot)) {
  throw new Error(
    `Astro fixture sources not found. Run: git submodule update --init ${plugin.submodule}`,
  );
}
const actualCommit = execFileSync('git', ['-C', upstreamRoot, 'rev-parse', 'HEAD'], {
  encoding: 'utf8',
}).trim();
if (actualCommit !== expectedCommit) {
  throw new Error(`Astro submodule must be ${expectedCommit}; found ${actualCommit}.`);
}

const ruleNames = [
  'no-deprecated-astro-canonicalurl',
  'no-deprecated-astro-fetchcontent',
  'no-deprecated-getentrybyslug',
] as const;
const rules: Record<string, { valid: FixtureCase[]; invalid: FixtureCase[] }> = {};
const sourceFiles: string[] = [];

for (const ruleName of ruleNames) {
  rules[ruleName] = {
    valid: readCases(ruleName, 'valid'),
    invalid: readCases(ruleName, 'invalid'),
  };
}

const outputDirectory = join(ROOT, 'npm', 'astro', 'test', 'fixtures');
mkdirSync(outputDirectory, { recursive: true });
const outputPath = join(outputDirectory, `astro-v${expectedVersion}.json`);
const fixture = {
  __generated: {
    source: plugin.npm,
    version: expectedVersion,
    ref: expectedRef,
    commit: expectedCommit,
    sourceFiles: sourceFiles.sort(),
    license: plugin.license,
    tool: 'tools/tasks/sync-astro-tests.ts',
  },
  rules,
};
writeFileSync(outputPath, `${JSON.stringify(fixture, null, 2)}\n`);

const caseCount = Object.values(rules).reduce(
  (count, cases) => count + cases.valid.length + cases.invalid.length,
  0,
);
console.log(
  `Synced ${caseCount} authored eslint-plugin-astro ${expectedRef} cases for ${ruleNames.length} rules.`,
);

function readCases(ruleName: string, validity: 'valid' | 'invalid'): FixtureCase[] {
  const directory = join(fixtureRoot, ruleName, validity);
  if (!existsSync(directory)) {
    throw new Error(`Missing upstream fixture directory: ${directory}`);
  }
  return readdirSync(directory)
    .filter((name) => name.endsWith('-input.astro'))
    .sort()
    .map((inputName) => {
      const stem = inputName.slice(0, -'-input.astro'.length);
      const inputPath = join(directory, inputName);
      sourceFiles.push(relative(upstreamRoot, inputPath));
      const fixtureCase: FixtureCase = {
        filename: basename(inputName),
        code: readFileSync(inputPath, 'utf8'),
      };
      if (validity === 'invalid') {
        const errorsPath = join(directory, `${stem}-errors.json`);
        if (!existsSync(errorsPath)) {
          throw new Error(`Missing authored errors for ${inputPath}`);
        }
        sourceFiles.push(relative(upstreamRoot, errorsPath));
        fixtureCase.errors = JSON.parse(readFileSync(errorsPath, 'utf8')) as AuthoredError[];
        const outputPath = join(directory, `${stem}-output.astro`);
        if (existsSync(outputPath)) {
          sourceFiles.push(relative(upstreamRoot, outputPath));
          fixtureCase.output = readFileSync(outputPath, 'utf8');
        }
      }
      return fixtureCase;
    });
}
