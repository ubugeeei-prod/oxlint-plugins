import statusSource from '../../../status.json?raw';

type Status = Array<{
  packageName: string;
  directory: string;
  version: string;
  published: boolean;
  publishedVersion: string | null;
  upstream: string | null;
  status: string;
  typeAware: boolean;
  rules: Array<{
    name: string;
    status: string;
    published: boolean;
    oxlintBuiltin: boolean;
    typeAware: boolean;
    rustCore: boolean;
    napi: boolean;
    tests: {
      insta: boolean;
      vitest: boolean;
      lsp: boolean;
      oxlintIntegration: boolean;
    };
    notes: string;
  }>;
}>;

const packages = JSON.parse(statusSource) as Status;

export function renderStatusMarkdown() {
  const rows = packages.flatMap((pkg) =>
    pkg.rules.map((rule) => {
      const tests = [
        rule.tests.insta ? 'insta' : null,
        rule.tests.vitest ? 'vitest' : null,
        rule.tests.lsp ? 'lsp' : null,
        rule.tests.oxlintIntegration ? 'oxlint' : null,
      ]
        .filter(Boolean)
        .join(', ');

      return [
        pkg.packageName,
        rule.name,
        rule.status,
        pkg.published ? `yes (${pkg.publishedVersion ?? pkg.version})` : 'no',
        rule.oxlintBuiltin ? 'yes' : 'no',
        rule.typeAware ? 'yes' : 'no',
        rule.rustCore ? 'yes' : 'no',
        rule.napi ? 'yes' : 'no',
        tests || 'none',
        rule.notes,
      ];
    }),
  );

  return `# Ruleset And Rule Status

This page is generated from \`status.json\`. Run \`vp run status:sync\` to refresh publish metadata from npm.

| Ruleset Package | Rule | Status | Published | Oxlint Builtin | Type-Aware | Rust Core | NAPI | Tests | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
${rows.map((row) => `| ${row.map(escapeMarkdownTable).join(' | ')} |`).join('\n')}
`;
}

function escapeMarkdownTable(value: string) {
  return value.replace(/\|/g, '\\|').replace(/\n/g, ' ');
}
