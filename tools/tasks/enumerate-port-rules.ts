// Enumerates every rule of each port-target ESLint plugin straight from the
// upstream source checked out under `upstream/` (git submodules), so the rule
// inventory cannot silently drift or miss a rule. Output is written to
// `docs/port-targets/`. Each plugin's enumerated count is asserted against the
// `expectedRuleCount` baked into `tools/port-targets.json`; a mismatch fails the
// run. Re-run with `pnpm run port:rules` (or `node tools/tasks/enumerate-port-rules.ts`).

import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

type GlobRules = {
  type: 'glob';
  dir: string;
  ext: string;
  exclude: string[];
};
type FixedRules = {
  type: 'fixed';
  dir: string;
  ext: string;
  names: string[];
};
type SonarRules = {
  type: 'sonarjs';
  dir: string;
  implementation: string;
  excludeEslintIds: string[];
};
type NoneRules = { type: 'none' };
type RulesConfig = GlobRules | FixedRules | SonarRules | NoneRules;

type Plugin = {
  id: string;
  npm: string;
  repo: string;
  submodule: string;
  packageSubdir: string;
  baselineVersion: string;
  pinnedRef: string;
  license: string;
  monorepo: boolean;
  expectedRuleCount: number;
  rules: RulesConfig;
  docsUrlTemplate: string | null;
  notes: string;
};
type Manifest = {
  submoduleRoot: string;
  plugins: Plugin[];
};

type Rule = {
  name: string;
  description: string;
  sonarKey?: string;
  docsUrl?: string;
};

const ROOT = process.cwd();
const MANIFEST_PATH = join(ROOT, 'tools', 'port-targets.json');
const OUT_DIR = join(ROOT, 'docs', 'port-targets');

const manifest = JSON.parse(readFileSync(MANIFEST_PATH, 'utf8')) as Manifest;

const errors: string[] = [];
const jsonPlugins: Array<{
  id: string;
  npm: string;
  repo: string;
  baselineVersion: string;
  license: string;
  ruleCount: number;
  rules: Rule[];
}> = [];

mkdirSync(OUT_DIR, { recursive: true });

for (const plugin of manifest.plugins) {
  const rules = enumerate(plugin);
  rules.sort((a, b) => a.name.localeCompare(b.name));

  if (rules.length !== plugin.expectedRuleCount) {
    errors.push(
      `${plugin.npm}: enumerated ${rules.length} rules but manifest expects ${plugin.expectedRuleCount}. ` +
        `Update tools/port-targets.json (expectedRuleCount / rules selector) or the submodule pin.`,
    );
  }

  writeFileSync(join(OUT_DIR, `${plugin.id}.md`), renderPluginDoc(plugin, rules));
  jsonPlugins.push({
    id: plugin.id,
    npm: plugin.npm,
    repo: plugin.repo,
    baselineVersion: plugin.baselineVersion,
    license: plugin.license,
    ruleCount: rules.length,
    rules,
  });

  console.log(`${plugin.npm.padEnd(48)} ${rules.length} rules`);
}

writeFileSync(
  join(OUT_DIR, 'rules.json'),
  `${JSON.stringify({ generatedFrom: 'tools/port-targets.json', plugins: jsonPlugins }, null, 2)}\n`,
);
writeFileSync(join(OUT_DIR, 'README.md'), renderIndex(manifest, jsonPlugins));

if (errors.length > 0) {
  console.error('\nRule enumeration mismatches:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log(`\nWrote ${jsonPlugins.length} plugin docs + rules.json to docs/port-targets/.`);

function enumerate(plugin: Plugin): Rule[] {
  const base = join(ROOT, plugin.submodule);
  const config = plugin.rules;

  if (config.type === 'none') return [];

  if (config.type === 'fixed') {
    return config.names.map((name) => {
      const file = join(base, config.dir, `${name}${config.ext}`);
      return makeRule(plugin, name, readDescription(file));
    });
  }

  if (config.type === 'sonarjs') {
    const dir = join(base, config.dir);
    if (!existsSync(dir)) {
      errors.push(`${plugin.npm}: rules dir not found: ${plugin.submodule}/${config.dir}`);
      return [];
    }
    const exclude = new Set(config.excludeEslintIds);
    const out: Rule[] = [];
    for (const entry of readdirSync(dir)) {
      if (!/^S\d+/.test(entry)) continue;
      const metaFile = join(dir, entry, 'meta.ts');
      if (!existsSync(metaFile)) continue;
      const text = readFileSync(metaFile, 'utf8');
      const eslintId = text.match(/export const eslintId = '([^']+)'/)?.[1];
      const implementation = text.match(/export const implementation = '([^']+)'/)?.[1];
      if (!eslintId || implementation !== config.implementation || exclude.has(eslintId)) continue;
      out.push(makeRule(plugin, eslintId, '', entry));
    }
    return out;
  }

  // glob
  const dir = join(base, config.dir);
  if (!existsSync(dir)) {
    errors.push(`${plugin.npm}: rules dir not found: ${plugin.submodule}/${config.dir}`);
    return [];
  }
  const out: Rule[] = [];
  for (const file of readdirSync(dir)) {
    if (!file.endsWith(config.ext)) continue;
    if (config.exclude.some((x) => file === x || file.endsWith(x))) continue;
    const name = file.slice(0, -config.ext.length);
    out.push(makeRule(plugin, name, readDescription(join(dir, file))));
  }
  return out;
}

function makeRule(plugin: Plugin, name: string, description: string, sonarKey?: string): Rule {
  const rule: Rule = { name, description };
  if (sonarKey) rule.sonarKey = sonarKey;
  const url = ruleDocsUrl(plugin, name, sonarKey);
  if (url) rule.docsUrl = url;
  return rule;
}

function ruleDocsUrl(plugin: Plugin, name: string, sonarKey?: string): string | undefined {
  const template = plugin.docsUrlTemplate;
  if (!template) return undefined;
  if (template.includes('{sonarKey}')) {
    return sonarKey ? template.replace('{sonarKey}', sonarKey) : undefined;
  }
  if (template.includes('{rule}')) return template.replace('{rule}', name);
  return undefined;
}

function readDescription(file: string): string {
  if (!existsSync(file)) return '';
  const src = readFileSync(file, 'utf8');
  // Prefer a description inside a `docs: { ... }` block, otherwise the first one.
  const docsIdx = src.indexOf('docs:');
  const haystack = docsIdx >= 0 ? src.slice(docsIdx) : src;
  const match = haystack.match(/description:\s*(['"`])((?:\\.|(?!\1)[\s\S])*?)\1/);
  if (!match) return '';
  return match[2].replace(/\s+/g, ' ').trim();
}

function renderPluginDoc(plugin: Plugin, rules: Rule[]): string {
  const lines: string[] = [];
  lines.push(
    `<!-- GENERATED by tools/tasks/enumerate-port-rules.ts from tools/port-targets.json + the ${plugin.submodule} submodule. Do not edit by hand; run \`pnpm run port:rules\`. -->`,
  );
  lines.push('');
  lines.push(`# Port target: \`${plugin.npm}\``);
  lines.push('');
  lines.push('| | |');
  lines.push('|---|---|');
  lines.push(`| Upstream repo | ${plugin.repo} |`);
  const subdir = plugin.packageSubdir === '.' ? '' : ` (\`${plugin.packageSubdir}\`)`;
  lines.push(`| Submodule | \`${plugin.submodule}\` @ \`${plugin.pinnedRef}\`${subdir} |`);
  lines.push(`| Baseline npm version | \`${plugin.baselineVersion}\` |`);
  lines.push(`| License | ${plugin.license} |`);
  lines.push(`| Oxlint native support | none — port target |`);
  lines.push(`| Rules to port | ${rules.length} |`);
  lines.push('');
  if (plugin.notes) {
    lines.push(`> ${plugin.notes}`);
    lines.push('');
  }
  lines.push('## Rules');
  lines.push('');

  const isSonar = plugin.rules.type === 'sonarjs';
  if (isSonar) {
    lines.push('| # | Rule | Sonar key | Spec |');
    lines.push('|---|------|-----------|------|');
    rules.forEach((rule, index) => {
      const spec = rule.docsUrl ? `[RSPEC](${rule.docsUrl})` : '';
      lines.push(`| ${index + 1} | \`${rule.name}\` | ${rule.sonarKey ?? ''} | ${spec} |`);
    });
  } else {
    const hasDocs = rules.some((rule) => rule.docsUrl);
    if (hasDocs) {
      lines.push('| # | Rule | Description | Docs |');
      lines.push('|---|------|-------------|------|');
      rules.forEach((rule, index) => {
        const docs = rule.docsUrl ? `[docs](${rule.docsUrl})` : '';
        lines.push(
          `| ${index + 1} | \`${rule.name}\` | ${escapeCell(rule.description)} | ${docs} |`,
        );
      });
    } else {
      lines.push('| # | Rule | Description |');
      lines.push('|---|------|-------------|');
      rules.forEach((rule, index) => {
        lines.push(`| ${index + 1} | \`${rule.name}\` | ${escapeCell(rule.description)} |`);
      });
    }
  }
  lines.push('');
  return `${lines.join('\n')}`;
}

function renderIndex(
  manifest: Manifest,
  plugins: Array<{ id: string; npm: string; repo: string; license: string; ruleCount: number }>,
): string {
  const lines: string[] = [];
  lines.push(
    '<!-- GENERATED by tools/tasks/enumerate-port-rules.ts. Do not edit by hand; run `pnpm run port:rules`. -->',
  );
  lines.push('');
  lines.push('# Port targets');
  lines.push('');
  lines.push(
    'ESLint plugins used by [flyle-nexus] that Oxlint does not support natively, collected here as port targets. ' +
      '`eslint-plugin-svelte` is intentionally excluded — it is handled by [rsvelte](https://github.com/baseballyama/rsvelte). ' +
      'Oxlint-supported plugins (`typescript-eslint`, `eslint-plugin-import`, `eslint-plugin-n`, `eslint-plugin-unicorn`) are used directly via Oxlint and are not listed here.',
  );
  lines.push('');
  lines.push(
    "Upstream source is vendored under `upstream/` as git submodules pinned to each plugin's baseline version. The per-rule inventory below is generated from that source.",
  );
  lines.push('');
  lines.push('| Plugin | Rules | License | Upstream |');
  lines.push('|--------|-------|---------|----------|');
  const total = plugins.reduce((sum, plugin) => sum + plugin.ruleCount, 0);
  for (const plugin of plugins) {
    lines.push(
      `| [\`${plugin.npm}\`](./${plugin.id}.md) | ${plugin.ruleCount} | ${plugin.license} | ${plugin.repo} |`,
    );
  }
  lines.push(`| **Total** | **${total}** | | |`);
  lines.push('');
  lines.push('[flyle-nexus]: internal');
  lines.push('');
  return lines.join('\n');
}

function escapeCell(value: string): string {
  return value.replace(/\|/g, '\\|').replace(/\n/g, ' ');
}
