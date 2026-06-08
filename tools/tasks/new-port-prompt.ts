import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

type PortTarget = {
  plugin: string;
  rule: string | null;
};

const rawTarget = process.argv[2];

if (!rawTarget) {
  console.error('Usage: vp run new <existing_plugin_name>[/rule_name]');
  console.error('Example: vp run new eslint-plugin-react/jsx-no-bind');
  process.exit(1);
}

const target = parseTarget(rawTarget);
const safeName = [target.plugin, target.rule]
  .filter((part): part is string => Boolean(part))
  .join('__')
  .replace(/[^a-zA-Z0-9._-]+/g, '_');
const outputDir = resolve('prompts/generated');
const outputPath = resolve(outputDir, `${safeName}.md`);
const prompt = renderPrompt(target);

mkdirSync(outputDir, { recursive: true });
writeFileSync(outputPath, prompt);

console.log(outputPath);

function parseTarget(input: string): PortTarget {
  const trimmed = input.trim();
  if (trimmed.length === 0) {
    throw new Error('Target must not be empty.');
  }

  if (trimmed.startsWith('@')) {
    const parts = trimmed.slice(1).split('/');
    if (parts.length < 2 || !parts[0] || !parts[1]) {
      throw new Error('Scoped targets must look like @scope/eslint-plugin-name[/rule-name].');
    }

    return {
      plugin: `@${parts[0]}/${parts[1]}`,
      rule: parts.slice(2).join('/') || null,
    };
  }

  const [plugin, ...ruleParts] = trimmed.split('/');
  return {
    plugin,
    rule: ruleParts.join('/') || null,
  };
}

function renderPrompt({ plugin, rule }: PortTarget): string {
  const packageStem = plugin
    .replace(/^@/, '')
    .replace(/^eslint-plugin-/, '')
    .replace(/\/eslint-plugin-/, '/')
    .replace(/[^a-zA-Z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
  const pluginPackageName = `@oxlint-plugins/oxlint-plugin-${packageStem}`;
  const scope = rule ? `rule ${rule}` : `the highest-value initial rule set`;

  return `# Port ${plugin}${rule ? `/${rule}` : ''}

You are working in the oxlint-plugins monorepo. Port ${scope} from ${plugin} to a Rust-backed Oxlint JS plugin distributed as ${pluginPackageName}.

## Constraints

- Keep the public integration compatible with Oxlint's JS plugin API.
- Put hot rule logic in Rust and expose it through NAPI-RS.
- Avoid per-node NAPI calls when a per-file Rust pre-scan or batch result can work.
- Use @corsa-bind/napi through @oxlint-plugins/oxlint-plugin-type-aware for type-aware rules.
- Avoid slow hash tables. Use oxlint_plugins_carton::FastHashMap/FastHashSet when maps are required.
- Avoid allocations in hot paths. Prefer borrowed data, CompactString, SmallVec, GhostCell, GhostToken, BumpAllocator, ArenaVec, ArenaString, and per-file reusable state from oxlint_plugins_carton.
- Treat String and Vec<T> as NAPI/public ABI types only; compact them immediately at the boundary before rule logic runs.
- Never call to_string in Rust hot paths. Use CompactString::from, borrowed data, or explicit ABI-boundary conversion.
- Prefer phf for static keyword/rule/message lookup tables.
- Do not add mod.rs; use named module files.
- Add Rust benchmarks for non-trivial hot logic and compile them in verification.
- Keep compatibility gaps explicit. Do not silently diverge from the upstream ESLint rule.
- PR titles must be conventional and must not mention Codex.
- Credit @ubugeeei and @baseballyama in new package metadata.
- Respect upstream licenses. Do not copy implementation code, tests, fixtures, docs, or messages unless the license permits it and attribution is preserved.

## Discovery

1. Inspect the upstream package source, tests, README, and rule docs for ${plugin}${rule ? `/${rule}` : ''}.
2. Record the supported ESLint version, parser assumptions, option schema, messages, fixes, suggestions, and known edge cases.
3. Check whether Oxlint already has a native builtin for this rule or a close equivalent. If yes, document whether this package should delegate, skip, or exist only for parity gaps.
4. Record upstream license and attribution requirements before writing code.

## Implementation

1. Add or update an installable package under npm/.
2. Keep the npm package individually installable and publishable.
3. Put reusable performance primitives in crates/_carton instead of ad hoc local types.
4. Put rule logic in a domain crate such as crates/stylistic, crates/import, crates/react, or crates/security. Do not create one Rust crate per rule.
5. Implement the Rust core with explicit data-flow boundaries and no std HashMap/HashSet.
6. Wrap the Rust core with @oxlint/plugins eslintCompatPlugin and createOnce.
7. Prefer before/after hooks for per-file state and return false from before when Rust can prove the rule is irrelevant to the file.
8. For type-aware behavior, initialize Corsa once, update snapshots by file version, and batch type queries outside AST hot visitors.

## Tests

Write more tests than seems necessary:

- Rust unit tests with insta snapshots for valid, invalid, options, edge boundaries, and regression cases.
- Rust property or table tests for parser-independent logic where possible.
- Vitest wrapper tests with snapshots for reports, options, and skip behavior.
- LSP diagnostic and quick-fix fixture tests for editor-facing behavior.
- Oxlint integration fixture tests when the current plugin API supports the scenario.
- Corsa-backed type-aware fixtures when the upstream rule requires types.
- Package dry-run checks so generated native files and package files are correct.
- Tests for upstream compatibility examples copied into fixtures with source attribution in comments.
- License policy checks for Rust and Node dependencies.

## Verification

Run these before handing off:

\`\`\`sh
vp fmt --check
cargo fmt --all --check
vp lint
cargo clippy --workspace --all-targets --all-features -- -D warnings
vp build
vp test
cargo test --workspace --all-features
vp run bench:check
pnpm run security:audit
node tools/tasks/verify-package-publish-ready.ts
pnpm run license
\`\`\`

## Deliverable

Return a concise summary with:

- Package and rule names.
- Upstream behavior covered.
- Known gaps.
- Test commands run.
- Any follow-up that should become a GitHub issue.
`;
}
