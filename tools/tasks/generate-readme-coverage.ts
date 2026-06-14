import { execFileSync } from 'node:child_process';
import { readFileSync, rmSync, writeFileSync } from 'node:fs';

// Regenerates the "Supported Plugins" coverage section of README.md from
// status.json (the single source of truth for which upstream ESLint plugins are
// ported and which of their rules are implemented). Run with `--check` in CI to
// fail when README.md is stale; run without flags to rewrite it in place.

type RuleStatus = 'ported' | 'pending' | 'approximation' | 'sample';

type RuleEntry = {
  name: string;
  status: RuleStatus;
};

type PluginEntry = {
  packageName: string;
  directory: string;
  upstream?: {
    package?: string | null;
    repository?: string | null;
  } | null;
  rules?: RuleEntry[];
};

const README_PATH = 'README.md';
const STATUS_PATH = 'status.json';
const BEGIN = '<!-- BEGIN GENERATED: plugin-coverage -->';
const END = '<!-- END GENERATED: plugin-coverage -->';

// A rule counts as implemented unless it is still pending. `approximation`
// (ported with documented divergences) and `sample` (the demo plugin) both
// ship working code, so they are counted as implemented.
const isImplemented = (rule: RuleEntry): boolean => rule.status !== 'pending';

const pluginName = (plugin: PluginEntry): string => plugin.directory.replace(/^npm\//, '');

const ruleList = (rules: RuleEntry[]): string =>
  rules
    .map((rule) => rule.name)
    .sort((a, b) => a.localeCompare(b))
    .map((name) => `\`${name}\``)
    .join(', ');

function render(plugins: PluginEntry[]): string {
  const sorted = [...plugins].sort((a, b) => pluginName(a).localeCompare(pluginName(b)));

  let totalRules = 0;
  let totalImplemented = 0;
  const rows: string[] = [];
  const sections: string[] = [];

  for (const plugin of sorted) {
    const rules = plugin.rules ?? [];
    const implemented = rules.filter(isImplemented);
    const pending = rules.filter((rule) => !isImplemented(rule));
    totalRules += rules.length;
    totalImplemented += implemented.length;

    const name = pluginName(plugin);
    const upstream = plugin.upstream?.package;
    const repo = plugin.upstream?.repository;
    const upstreamCell = upstream ? (repo ? `[\`${upstream}\`](${repo})` : `\`${upstream}\``) : '—';
    const pct = rules.length === 0 ? 0 : Math.round((implemented.length / rules.length) * 100);
    rows.push(
      `| [\`${name}\`](${plugin.directory}) | ${upstreamCell} | ${implemented.length} | ${rules.length} | ${pct}% |`,
    );

    const parts: string[] = [
      `<details>`,
      `<summary><code>${name}</code> — ${implemented.length}/${rules.length} implemented</summary>`,
      ``,
    ];
    if (implemented.length > 0) {
      parts.push(`**Implemented (${implemented.length}):** ${ruleList(implemented)}`, ``);
    }
    if (pending.length > 0) {
      parts.push(`**Not implemented (${pending.length}):** ${ruleList(pending)}`, ``);
    }
    parts.push(`</details>`);
    sections.push(parts.join('\n'));
  }

  const overallPct = totalRules === 0 ? 0 : Math.round((totalImplemented / totalRules) * 100);

  return [
    BEGIN,
    `## Supported Plugins`,
    ``,
    `<!-- This section is generated from \`status.json\` by \`tools/tasks/generate-readme-coverage.ts\`. Do not edit by hand; run \`pnpm run docs:readme\`. -->`,
    ``,
    `**${sorted.length}** ESLint plugins are being ported · **${totalImplemented} / ${totalRules}** rules implemented (**${overallPct}%**).`,
    ``,
    `| Plugin | Upstream | Implemented | Total | Coverage |`,
    `| --- | --- | --- | --- | --- |`,
    ...rows,
    ``,
    ...sections,
    ``,
    END,
  ].join('\n');
}

const check = process.argv.includes('--check');
const plugins = JSON.parse(readFileSync(STATUS_PATH, 'utf8')) as PluginEntry[];
const readme = readFileSync(README_PATH, 'utf8');
const section = render(plugins);

const beginIdx = readme.indexOf(BEGIN);
const endIdx = readme.indexOf(END);
if (beginIdx === -1 || endIdx === -1) {
  console.error(
    `README.md is missing the coverage markers (${BEGIN} … ${END}). Add them where the section should appear.`,
  );
  process.exit(1);
}

const next = readme.slice(0, beginIdx) + section + readme.slice(endIdx + END.length);

// Run the repo formatter (oxfmt via Vite+) so the generated section matches the
// project's Markdown style (table-column padding, blank lines). This keeps the
// generator's output identical to what `vp fmt --check` expects, so the README
// never oscillates between this task and the Format CI check.
function format(path: string): void {
  execFileSync('node_modules/.bin/vp', ['fmt', path], { stdio: 'ignore' });
}

if (check) {
  // Generate into a throwaway copy, format it the same way `docs:readme` would,
  // and compare against the committed README.
  const tmp = '.readme-coverage-check.md';
  writeFileSync(tmp, next);
  try {
    format(tmp);
    const formatted = readFileSync(tmp, 'utf8');
    if (formatted !== readme) {
      console.error(
        'README.md plugin-coverage section is out of date. Run `pnpm run docs:readme`.',
      );
      process.exit(1);
    }
  } finally {
    rmSync(tmp, { force: true });
  }
  console.log('README.md plugin-coverage section is up to date.');
} else {
  writeFileSync(README_PATH, next);
  format(README_PATH);
  console.log('Updated README.md plugin-coverage section.');
}
