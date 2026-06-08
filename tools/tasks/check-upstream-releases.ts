// Checks every port target in tools/port-targets.json against the npm registry
// and, when an upstream has published a version newer than the baseline we
// pinned, opens (or reuses) a GitHub issue so the submodule + port can follow
// up. Reads only the committed manifest and the public registry, so it needs
// neither the submodules nor pnpm/vp — just Node (global fetch) and the `gh`
// CLI for issue creation.
//
// Usage:
//   node tools/tasks/check-upstream-releases.ts            # dry run (prints)
//   node tools/tasks/check-upstream-releases.ts --apply    # create issues (CI)
//
// The scheduled workflow .github/workflows/track-upstream-releases.yml runs it
// with --apply and GH_TOKEN set.

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

type Plugin = {
  id: string;
  npm: string;
  repo: string;
  submodule: string;
  baselineVersion: string;
  pinnedRef: string;
  license: string;
};
type Manifest = { plugins: Plugin[] };

type ExistingIssue = { title: string; number: number; state: string };

const apply = process.argv.includes('--apply');
const repoSlug = process.env.GITHUB_REPOSITORY ?? 'ubugeeei-prod/oxlint-plugins';

const manifest = JSON.parse(
  readFileSync(join(process.cwd(), 'tools', 'port-targets.json'), 'utf8'),
) as Manifest;

const existing = apply ? listExistingIssues() : [];
const filed: string[] = [];
const failed: string[] = [];

for (const plugin of manifest.plugins) {
  const latest = await fetchLatestVersion(plugin.npm);
  if (!latest) {
    console.log(`? ${plugin.npm}: could not resolve latest version`);
    continue;
  }
  if (!isNewer(latest, plugin.baselineVersion)) {
    console.log(
      `= ${plugin.npm}: up to date (baseline ${plugin.baselineVersion}, latest ${latest})`,
    );
    continue;
  }

  const marker = `${plugin.npm}@${latest}`;
  console.log(`+ ${plugin.npm}: ${plugin.baselineVersion} -> ${latest} (follow-up needed)`);

  if (!apply) {
    filed.push(marker);
    continue;
  }

  if (existing.some((issue) => issue.title.includes(marker))) {
    console.log(`  issue already exists for ${marker}, skipping`);
    continue;
  }

  try {
    const url = createIssue(plugin, latest);
    filed.push(url);
    console.log(`  filed ${url}`);
  } catch (error) {
    failed.push(`${marker}: ${(error as Error).message.split('\n')[0]}`);
    console.error(`  failed to file issue for ${marker}`);
  }
}

console.log(
  `\n${apply ? 'Filed' : 'Would file'} ${filed.length} follow-up issue(s).` +
    (failed.length > 0 ? ` ${failed.length} failed.` : ''),
);
if (failed.length > 0) {
  for (const failure of failed) console.error(`- ${failure}`);
  process.exit(1);
}

async function fetchLatestVersion(npm: string): Promise<string | null> {
  try {
    const response = await fetch(`https://registry.npmjs.org/${npm}/latest`, {
      headers: { accept: 'application/json' },
    });
    if (!response.ok) return null;
    const body = (await response.json()) as { version?: string };
    return body.version ?? null;
  } catch {
    return null;
  }
}

function isNewer(candidate: string, baseline: string): boolean {
  const a = parseVersion(candidate);
  const b = parseVersion(baseline);
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) return a[i] > b[i];
  }
  return false;
}

function parseVersion(version: string): [number, number, number] {
  const match = String(version).match(/(\d+)\.(\d+)\.(\d+)/);
  if (!match) return [0, 0, 0];
  return [Number(match[1]), Number(match[2]), Number(match[3])];
}

function listExistingIssues(): ExistingIssue[] {
  try {
    const out = execFileSync(
      'gh',
      [
        'issue',
        'list',
        '--repo',
        repoSlug,
        '--label',
        'upstream-release',
        '--state',
        'all',
        '--limit',
        '300',
        '--json',
        'title,number,state',
      ],
      { encoding: 'utf8' },
    );
    return JSON.parse(out) as ExistingIssue[];
  } catch {
    return [];
  }
}

function createIssue(plugin: Plugin, latest: string): string {
  const title = `upstream release: ${plugin.npm}@${latest} (baseline ${plugin.baselineVersion})`;
  const body = [
    `**${plugin.npm}** published \`${latest}\`. The port baseline in \`tools/port-targets.json\` is \`${plugin.baselineVersion}\`, so the vendored submodule and the port may need to follow up.`,
    '',
    `- Upstream repo: ${plugin.repo}`,
    `- Releases: ${plugin.repo}/releases`,
    `- Submodule: \`${plugin.submodule}\` (pinned to \`${plugin.pinnedRef}\`)`,
    `- License: ${plugin.license}`,
    '',
    '### Follow-up',
    '',
    '- [ ] Review the upstream changelog and the added / removed / changed rules',
    `- [ ] Bump the submodule and update \`baselineVersion\` + \`pinnedRef\` for \`${plugin.id}\` in \`tools/port-targets.json\``,
    '- [ ] Re-run `pnpm run port:rules` and reconcile `expectedRuleCount`',
    '- [ ] Port new rules / adjust changed behavior',
    '',
    '_Filed automatically by `.github/workflows/track-upstream-releases.yml`._',
  ].join('\n');

  return execFileSync(
    'gh',
    [
      'issue',
      'create',
      '--repo',
      repoSlug,
      '--title',
      title,
      '--body',
      body,
      '--label',
      'upstream-release',
    ],
    { encoding: 'utf8' },
  ).trim();
}
