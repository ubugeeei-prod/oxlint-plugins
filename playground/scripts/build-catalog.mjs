// Builds playground/src/catalog.json: per-rule metadata (descriptions, message
// templates, docs URLs) for every plugin the WASM module can run.
//
// Rule names come from the compiled WASM module (the source of truth for what
// is actually implemented). The human-facing metadata is read from each npm
// plugin's index.js by requiring it with light stubs for `@oxlint/plugins` and
// the native binding, so no native build is needed.
import Module from 'node:module';
import { createRequire } from 'node:module';
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, resolve, basename } from 'node:path';

const here = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(here, '..', '..');
const npmDir = resolve(repoRoot, 'npm');
const wasmJs = resolve(here, '..', 'src', 'wasm', 'oxlint_plugins_playground_wasm.js');
const wasmBin = resolve(here, '..', 'src', 'wasm', 'oxlint_plugins_playground_wasm_bg.wasm');
const outFile = resolve(here, '..', 'src', 'catalog.json');

// Rule names per plugin, from the WASM module.
const wasm = await import(pathToFileURL(wasmJs).href);
await wasm.default({ module_or_path: readFileSync(wasmBin) });
/** @type {{plugin: string, rules: string[]}[]} */
const wasmPlugins = JSON.parse(wasm.list_rules());

// Stub `@oxlint/plugins` and the native/api bindings while requiring index.js.
let currentRuleNames = [];
const nativeStub = new Proxy(
  {},
  {
    get(_target, prop) {
      if (typeof prop !== 'string') return undefined;
      if (prop.endsWith('RuleNames')) return () => currentRuleNames.slice();
      return () => [];
    },
  },
);
const pluginsStub = new Proxy(
  {},
  {
    get(_target, prop) {
      // `eslintCompatPlugin` and any other helper: return the config untouched
      // so we can read rule `meta` straight off it.
      if (prop === 'default') return pluginsStub;
      return (value) => value;
    },
  },
);

const originalLoad = Module._load;
Module._load = function patchedLoad(request, parent, isMain) {
  if (request === '@oxlint/plugins') return pluginsStub;
  if (request.endsWith('/native.js') || request.endsWith('/api.js')) return nativeStub;
  return originalLoad.call(this, request, parent, isMain);
};

const require = createRequire(import.meta.url);

function loadPlugin(dir, ruleNames) {
  const indexPath = resolve(npmDir, dir, 'index.js');
  currentRuleNames = ruleNames;
  delete require.cache[indexPath];
  return require(indexPath);
}

// First pass: map each plugin's reported name to its npm directory.
const dirs = wasmPlugins.length ? readDirsWithIndex() : [];
const nameToDir = new Map();
for (const dir of dirs) {
  try {
    const plugin = loadPlugin(dir, []);
    // A plugin's runtime name can differ from the short name the rules are
    // registered under (which is what the WASM module reports). Map every
    // candidate: the config's plugin prefix, the meta namespace, and meta.name.
    for (const key of pluginNameCandidates(plugin)) nameToDir.set(key, dir);
  } catch {
    // Skip plugins that cannot be required with stubs.
  }
}

function pluginNameCandidates(plugin) {
  const candidates = new Set();
  const configs = plugin?.configs ?? {};
  for (const config of Object.values(configs)) {
    const prefix = config?.plugins?.[0];
    if (typeof prefix === 'string') candidates.add(prefix);
  }
  if (typeof plugin?.meta?.namespace === 'string') candidates.add(plugin.meta.namespace);
  if (typeof plugin?.meta?.name === 'string') candidates.add(plugin.meta.name);
  return candidates;
}

function readDirsWithIndex() {
  const { readdirSync, existsSync } = require('node:fs');
  return readdirSync(npmDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .filter((name) => existsSync(resolve(npmDir, name, 'index.js')));
}

// Stylistic renders messages in Rust, so its templates come from the WASM
// module rather than from index.js.
const stylisticMetas = new Map();
try {
  for (const meta of JSON.parse(wasm.stylistic_rule_metas())) {
    stylisticMetas.set(meta.name, {
      description: typeof meta.docsDescription === 'string' ? meta.docsDescription : '',
      messages: meta.messages && typeof meta.messages === 'object' ? meta.messages : {},
    });
  }
} catch {
  // Leave stylistic metadata empty if the export is unavailable.
}

// Second pass: read rule metadata using the WASM-authoritative rule names.
const catalogPlugins = [];
for (const { plugin: pluginName, rules } of wasmPlugins) {
  if (pluginName === 'stylistic') {
    catalogPlugins.push({
      plugin: pluginName,
      npm: '@oxlint-plugins/oxlint-plugin-stylistic',
      description: '',
      rules: rules.map((name) => {
        const meta = stylisticMetas.get(name) ?? { description: '', messages: {} };
        return { name, description: meta.description, docsUrl: null, messages: meta.messages };
      }),
    });
    continue;
  }
  const dir = nameToDir.get(pluginName);
  if (!dir) {
    catalogPlugins.push({
      plugin: pluginName,
      npm: null,
      description: '',
      rules: rules.map((name) => ({ name, description: '', docsUrl: null, messages: {} })),
    });
    continue;
  }
  let pkg = {};
  try {
    pkg = JSON.parse(readFileSync(resolve(npmDir, dir, 'package.json'), 'utf8'));
  } catch {
    pkg = {};
  }
  const plugin = loadPlugin(dir, rules);
  const ruleEntries = rules.map((name) => {
    const meta = plugin?.rules?.[name]?.meta ?? {};
    const docs = meta.docs ?? {};
    return {
      name,
      description: typeof docs.description === 'string' ? docs.description : '',
      docsUrl: typeof docs.url === 'string' ? docs.url : null,
      messages: meta.messages && typeof meta.messages === 'object' ? meta.messages : {},
    };
  });
  catalogPlugins.push({
    plugin: pluginName,
    npm: typeof pkg.name === 'string' ? pkg.name : null,
    description: typeof pkg.description === 'string' ? pkg.description : '',
    rules: ruleEntries,
  });
}

Module._load = originalLoad;

catalogPlugins.sort((a, b) => a.plugin.localeCompare(b.plugin));
const catalog = {
  generatedFrom: 'npm/*/index.js + crates/playground_wasm',
  plugins: catalogPlugins,
};
writeFileSync(outFile, `${JSON.stringify(catalog, null, 2)}\n`);

const ruleCount = catalogPlugins.reduce((total, plugin) => total + plugin.rules.length, 0);
console.log(`Wrote ${basename(outFile)}: ${catalogPlugins.length} plugins, ${ruleCount} rules.`);
