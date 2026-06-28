import './styles.css';
import { EditorView, basicSetup } from 'codemirror';
import { Compartment, StateEffect, type Text } from '@codemirror/state';
import {
  linter,
  lintGutter,
  forceLinting,
  type Diagnostic as CmDiagnostic,
} from '@codemirror/lint';
import { javascript } from '@codemirror/lang-javascript';
import { json } from '@codemirror/lang-json';
import { markdown } from '@codemirror/lang-markdown';
import init, {
  lint,
  list_rules,
  language_for_filename,
} from './wasm/oxlint_plugins_playground_wasm.js';
import wasmUrl from './wasm/oxlint_plugins_playground_wasm_bg.wasm?url';
import catalogData from './catalog.json';
import { samples, type Sample } from './samples';

type RuleMeta = {
  name: string;
  description: string;
  docsUrl: string | null;
  messages: Record<string, string>;
};

type CatalogPlugin = {
  plugin: string;
  npm: string | null;
  description: string;
  rules: RuleMeta[];
};

type Catalog = { plugins: CatalogPlugin[] };

type PluginListing = {
  plugin: string;
  language: string;
  rules: string[];
};

type Diagnostic = {
  plugin: string;
  rule: string;
  message_id: string;
  data: Record<string, string>;
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
};

const catalog = catalogData as Catalog;

// Quick lookup for rule metadata, keyed by `plugin/rule` for O(1) access.
const ruleMeta = new Map<string, RuleMeta>();
for (const plugin of catalog.plugins) {
  for (const rule of plugin.rules) {
    ruleMeta.set(`${plugin.plugin}/${rule.name}`, rule);
  }
}

const LANGUAGE_LABEL: Record<string, string> = {
  javascript: 'JS / TS',
  json: 'JSON',
  markdown: 'MD',
};

const app = document.querySelector<HTMLDivElement>('#app');
if (!app) throw new Error('Missing #app root element.');
app.innerHTML = '<div class="loading">Loading the linter…</div>';

try {
  await init({ module_or_path: wasmUrl });
} catch (error) {
  console.error('Failed to load the playground WebAssembly module:', error);
  app.innerHTML =
    '<div class="loading">Failed to load the linter. Check the browser console for details.</div>';
  throw error;
}

const listing = JSON.parse(list_rules()) as PluginListing[];
listing.sort((a, b) => a.plugin.localeCompare(b.plugin));

// Enabled state: plugin -> set of enabled rule names. Everything starts on.
const enabled = new Map<string, Set<string>>();
for (const plugin of listing) {
  enabled.set(plugin.plugin, new Set(plugin.rules));
}

const totalRules = listing.reduce((sum, plugin) => sum + plugin.rules.length, 0);

// Shareable state lives in the URL hash so a bug report can link to the exact
// code, file name, and rule selection. Default is "everything enabled", so we
// only persist the rules that are turned off.
type SharedState = { f?: string; c?: string; off?: Record<string, string[]> };

function toBase64Url(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let binary = '';
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function fromBase64Url(value: string): string {
  const padded = value.replace(/-/g, '+').replace(/_/g, '/');
  const binary = atob(padded);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

function decodeSharedState(): SharedState | null {
  const raw = location.hash.replace(/^#/, '');
  if (!raw) return null;
  try {
    const parsed = JSON.parse(fromBase64Url(raw)) as SharedState;
    return parsed && typeof parsed === 'object' ? parsed : null;
  } catch {
    return null;
  }
}

const shared = decodeSharedState();
if (shared?.off) {
  for (const [plugin, rules] of Object.entries(shared.off)) {
    const set = enabled.get(plugin);
    if (!set || !Array.isArray(rules)) continue;
    for (const rule of rules) set.delete(rule);
  }
}

let filename =
  (typeof shared?.f === 'string' && shared.f.trim()) || samples[0]?.filename || 'example.js';
const initialCode = typeof shared?.c === 'string' ? shared.c : (samples[0]?.code ?? '');
let search = '';

// ---- DOM helpers ---------------------------------------------------------

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  props: Partial<HTMLElementTagNameMap[K]> & { class?: string } = {},
  children: (Node | string)[] = [],
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  const { class: className, ...rest } = props;
  if (className) node.className = className;
  Object.assign(node, rest);
  for (const child of children) {
    node.append(typeof child === 'string' ? document.createTextNode(child) : child);
  }
  return node;
}

// ---- Message rendering ---------------------------------------------------

function renderMessage(diagnostic: Diagnostic): string {
  const meta = ruleMeta.get(`${diagnostic.plugin}/${diagnostic.rule}`);
  const template = meta?.messages[diagnostic.message_id];
  if (!template) {
    // Fall back to the raw value passed by the rule (some rules carry a full
    // message string), then to the message id.
    return diagnostic.data.message ?? diagnostic.message_id;
  }
  // Drop any placeholder the adapter didn't populate rather than leaking a raw
  // `{{token}}` to the user.
  return template.replace(/\{\{\s*(\w+)\s*\}\}/g, (_match, key: string) =>
    key in diagnostic.data ? diagnostic.data[key] : '',
  );
}

// ---- Build the shell -----------------------------------------------------

app.innerHTML = '';

const editorHost = el('div', { class: 'editor-host' });
const diagList = el('div', { class: 'diag-list' });
const diagCount = el('span', { class: 'diag-count' });
const treeEl = el('div', { class: 'tree' });

const filenameInput = el('input', { value: filename, spellcheck: false });
filenameInput.setAttribute('aria-label', 'File name (its extension picks the language)');

// A leading placeholder keeps the control a one-shot picker: it always starts
// unselected (so a shared link's custom code isn't overwritten), and resetting
// to it after each load lets re-picking the same sample fire `change` again.
const sampleSelect = el('select', {}, [
  el('option', { value: '' }, ['Load sample…']),
  ...samples.map((sample, index) => el('option', { value: String(index) }, [sample.label])),
]);
sampleSelect.setAttribute('aria-label', 'Load a sample');

const searchInput = el('input', { class: 'search', type: 'search', placeholder: 'Filter rules…' });
searchInput.setAttribute('aria-label', 'Filter rules');

app.append(
  el('div', { class: 'app' }, [
    el('header', { class: 'topbar' }, [
      el('div', { class: 'brand' }, [
        el('span', { class: 'brand-mark' }, ['ox']),
        el('span', { class: 'brand-name' }, ['oxlint-plugins playground']),
        el('span', { class: 'brand-tag' }, [
          `${totalRules} rules · ${listing.length} packages · runs in your browser via `,
          el('code', {}, ['Rust→Wasm']),
        ]),
      ]),
      el('div', { class: 'controls' }, [
        el('label', { class: 'control' }, ['Sample', sampleSelect]),
        el('label', { class: 'control' }, ['File', filenameInput]),
      ]),
    ]),
    el('div', { class: 'main' }, [
      el('section', { class: 'panel sidebar' }, [
        el('div', { class: 'panel-head' }, [
          el('span', { class: 'panel-title' }, ['Rules']),
          el('div', { class: 'bulk' }, [
            buttonLink('Enable all', () => setAll(true)),
            buttonLink('Disable all', () => setAll(false)),
          ]),
        ]),
        el('div', { class: 'sidebar-tools' }, [searchInput]),
        treeEl,
      ]),
      el('section', { class: 'panel editor' }, [
        el('div', { class: 'panel-head' }, [el('span', { class: 'panel-title' }, ['Source'])]),
        editorHost,
      ]),
      el('section', { class: 'panel diagnostics' }, [
        el('div', { class: 'panel-head' }, [
          el('span', { class: 'panel-title' }, ['Problems']),
          diagCount,
        ]),
        diagList,
      ]),
    ]),
  ]),
);

function buttonLink(label: string, onClick: () => void): HTMLButtonElement {
  const button = el('button', { class: 'linkbtn', type: 'button' }, [label]);
  button.addEventListener('click', onClick);
  return button;
}

// ---- CodeMirror editor with inline diagnostics ---------------------------

const languageConf = new Compartment();

function languageExtension(name: string) {
  // The WASM module owns the authoritative extension → language map; the editor
  // only adds the jsx/typescript flags CodeMirror needs.
  const language = language_for_filename(name);
  if (language === 'json') return json();
  if (language === 'markdown') return markdown();
  const ext = name.includes('.') ? name.slice(name.lastIndexOf('.') + 1).toLowerCase() : '';
  return javascript({
    jsx: ext === 'jsx' || ext === 'tsx',
    typescript: ext === 'ts' || ext === 'tsx' || ext === 'cts' || ext === 'mts',
  });
}

function offsetFor(doc: Text, line: number, column: number): number {
  const lineNumber = Math.min(Math.max(line, 1), doc.lines);
  const lineInfo = doc.line(lineNumber);
  return Math.min(lineInfo.from + Math.max(column, 0), lineInfo.to);
}

// The document offset range [from, to] a diagnostic covers.
function rangeFor(doc: Text, diagnostic: Diagnostic): { from: number; to: number } {
  const from = offsetFor(doc, diagnostic.start_line, diagnostic.start_column);
  const to = Math.max(from, offsetFor(doc, diagnostic.end_line, diagnostic.end_column));
  return { from, to };
}

type LintResult = { diagnostics: Diagnostic[]; error: string | null };

// The single source of truth for diagnostics: the WASM linter. CodeMirror calls
// this on edits (and we force it on toggles), and we mirror the results into the
// Problems panel so the inline markers and the list always agree.
function computeDiagnostics(doc: Text): LintResult {
  const enabledJson = buildEnabledJson();
  if (enabledJson === '') return { diagnostics: [], error: null };
  try {
    const diagnostics = JSON.parse(lint(doc.toString(), filename, enabledJson)) as Diagnostic[];
    return { diagnostics, error: null };
  } catch (error) {
    return { diagnostics: [], error: error instanceof Error ? error.message : String(error) };
  }
}

// Lets us re-run the linter when the enabled rules or filename change, since
// those aren't document edits that would otherwise trigger CodeMirror.
const refreshLint = StateEffect.define<null>();

const wasmLinter = linter(
  (view) => {
    const result = computeDiagnostics(view.state.doc);
    result.diagnostics.sort(
      (a, b) => a.start_line - b.start_line || a.start_column - b.start_column,
    );
    // Render each message once and reuse it for both the panel and the markers.
    const messages = result.diagnostics.map(renderMessage);
    renderProblems(view, result, messages);
    if (result.error) {
      console.error('Playground lint failed:', result.error);
      return [];
    }
    return result.diagnostics.map((diagnostic, index): CmDiagnostic => {
      const { from, to } = rangeFor(view.state.doc, diagnostic);
      return {
        from,
        to,
        severity: 'error',
        source: `${diagnostic.plugin}/${diagnostic.rule}`,
        message: messages[index],
      };
    });
  },
  {
    delay: 150,
    needsRefresh: (update) =>
      update.transactions.some((tr) => tr.effects.some((effect) => effect.is(refreshLint))),
  },
);

const editorTheme = EditorView.theme({
  '&': { height: '100%', fontSize: '13px' },
  '.cm-scroller': {
    fontFamily: "'IBM Plex Mono', ui-monospace, SFMono-Regular, Menlo, monospace",
    lineHeight: '1.6',
  },
  '.cm-gutters': { background: '#f1f4f9', border: 'none', color: '#9aa4b6' },
  '&.cm-focused': { outline: 'none' },
});

const view = new EditorView({
  parent: editorHost,
  doc: initialCode,
  extensions: [
    basicSetup,
    languageConf.of(languageExtension(filename)),
    lintGutter(),
    wasmLinter,
    editorTheme,
    EditorView.lineWrapping,
    EditorView.updateListener.of((update) => {
      if (update.docChanged) scheduleHashUpdate();
    }),
  ],
});

function relint(): void {
  view.dispatch({ effects: refreshLint.of(null) });
  forceLinting(view);
  scheduleHashUpdate();
}

// Persist the current code, file name, and disabled rules into the URL hash.
function currentSharedState(): SharedState {
  const off: Record<string, string[]> = {};
  for (const plugin of listing) {
    const set = enabled.get(plugin.plugin);
    if (!set) continue;
    const disabled = plugin.rules.filter((rule) => !set.has(rule));
    if (disabled.length > 0) off[plugin.plugin] = disabled;
  }
  const state: SharedState = { f: filename, c: view.state.doc.toString() };
  if (Object.keys(off).length > 0) state.off = off;
  return state;
}

let hashTimer = 0;
function scheduleHashUpdate(): void {
  window.clearTimeout(hashTimer);
  hashTimer = window.setTimeout(() => {
    const encoded = toBase64Url(JSON.stringify(currentSharedState()));
    history.replaceState(null, '', `#${encoded}`);
  }, 200);
}

// ---- Sidebar (rules tree) ------------------------------------------------

// Packages start collapsed so the 600+ rules don't overwhelm on first paint.
const collapsed = new Set<string>(listing.map((plugin) => plugin.plugin));

function renderTree(): void {
  const query = search.trim().toLowerCase();
  treeEl.innerHTML = '';
  let shown = 0;

  for (const plugin of listing) {
    const matchingRules = plugin.rules.filter((rule) => {
      if (!query) return true;
      const description = ruleMeta.get(`${plugin.plugin}/${rule}`)?.description ?? '';
      return (
        rule.toLowerCase().includes(query) ||
        plugin.plugin.toLowerCase().includes(query) ||
        description.toLowerCase().includes(query)
      );
    });
    if (matchingRules.length === 0) continue;
    shown += 1;

    const enabledSet = enabled.get(plugin.plugin) ?? new Set<string>();
    const onCount = matchingRules.filter((rule) => enabledSet.has(rule)).length;
    const allOn = onCount === matchingRules.length;
    const someOn = onCount > 0;
    const state = allOn ? 'on' : someOn ? 'partial' : 'off';

    const isCollapsed = collapsed.has(plugin.plugin) && !query;
    const pkg = el('div', { class: someOn ? 'pkg' : 'pkg pkg-off' });
    if (isCollapsed) pkg.classList.add('collapsed');

    // The whole header row expands/collapses; the switch on the right toggles
    // every rule in the package.
    const toggle = el('button', { class: 'switch', type: 'button' });
    toggle.dataset.state = state;
    toggle.setAttribute('role', 'switch');
    toggle.setAttribute('aria-checked', allOn ? 'true' : someOn ? 'mixed' : 'false');
    toggle.setAttribute('aria-label', `Toggle all ${plugin.plugin} rules`);
    toggle.append(el('span', { class: 'switch-knob' }));
    toggle.addEventListener('click', (event) => {
      event.stopPropagation();
      const turnOn = !allOn;
      for (const rule of matchingRules) {
        if (turnOn) enabledSet.add(rule);
        else enabledSet.delete(rule);
      }
      renderTree();
      relint();
    });

    const head = el('button', { class: 'pkg-head', type: 'button' }, [
      el('span', { class: 'chevron' }, ['▸']),
      el('span', { class: 'pkg-name' }, [plugin.plugin]),
      el('span', { class: 'pkg-lang' }, [LANGUAGE_LABEL[plugin.language] ?? plugin.language]),
      el('span', { class: 'pkg-count' }, [`${onCount}/${matchingRules.length}`]),
      toggle,
    ]);
    head.setAttribute('aria-expanded', String(!isCollapsed));
    head.addEventListener('click', () => {
      if (collapsed.has(plugin.plugin)) collapsed.delete(plugin.plugin);
      else collapsed.add(plugin.plugin);
      renderTree();
    });

    const rulesEl = el('div', { class: 'rules' });
    for (const rule of matchingRules) {
      const on = enabledSet.has(rule);
      const ruleInfo = ruleMeta.get(`${plugin.plugin}/${rule}`);
      // The whole row is the toggle. The checkbox is a visual indicator only
      // (clicks/keyboard go through the row) so there is one source of truth.
      const row = el('div', {
        class: on ? 'rule' : 'rule rule-off',
        title: ruleInfo?.description ?? '',
        tabIndex: 0,
      });
      row.setAttribute('role', 'checkbox');
      row.setAttribute('aria-checked', String(on));
      const checkbox = el('input', { type: 'checkbox', checked: on, tabIndex: -1 });
      checkbox.setAttribute('aria-hidden', 'true');
      const toggleRule = () => {
        if (enabledSet.has(rule)) enabledSet.delete(rule);
        else enabledSet.add(rule);
        renderTree();
        relint();
      };
      row.addEventListener('click', toggleRule);
      row.addEventListener('keydown', (event) => {
        if (event.key === ' ' || event.key === 'Enter') {
          event.preventDefault();
          toggleRule();
        }
      });
      row.append(checkbox, el('span', { class: 'rule-name' }, [rule]));
      if (ruleInfo?.docsUrl) {
        const link = el(
          'a',
          {
            class: 'rule-doc',
            href: ruleInfo.docsUrl,
            target: '_blank',
            rel: 'noreferrer',
            title: 'Open documentation',
          },
          ['↗'],
        );
        link.addEventListener('click', (event) => event.stopPropagation());
        row.append(link);
      }
      rulesEl.append(row);
    }

    pkg.append(head, rulesEl);
    treeEl.append(pkg);
  }

  if (shown === 0) {
    treeEl.append(el('div', { class: 'empty' }, ['No rules match your filter.']));
  }
}

function setAll(value: boolean): void {
  for (const plugin of listing) {
    const set = enabled.get(plugin.plugin);
    if (!set) continue;
    set.clear();
    if (value) for (const rule of plugin.rules) set.add(rule);
  }
  renderTree();
  relint();
}

// ---- Problems panel ------------------------------------------------------

function buildEnabledJson(): string {
  const payload: Record<string, true | string[]> = {};
  let any = false;
  for (const plugin of listing) {
    const set = enabled.get(plugin.plugin);
    if (!set || set.size === 0) continue;
    any = true;
    payload[plugin.plugin] = set.size === plugin.rules.length ? true : [...set];
  }
  return any ? JSON.stringify(payload) : '';
}

function renderProblems(editor: EditorView, result: LintResult, messages: string[]): void {
  const { diagnostics, error } = result;
  diagList.innerHTML = '';
  if (error) {
    diagCount.textContent = 'error';
    diagList.append(
      el('div', { class: 'empty' }, [
        el('strong', {}, ['The linter failed to run.']),
        el('div', { class: 'diag-error' }, [error]),
      ]),
    );
    return;
  }
  diagCount.textContent = `${diagnostics.length} ${diagnostics.length === 1 ? 'problem' : 'problems'}`;
  if (diagnostics.length === 0) {
    diagList.append(
      el('div', { class: 'empty' }, [
        el('strong', {}, ['No problems found.']),
        el('div', {}, ['Edit the code or enable more rules.']),
      ]),
    );
    return;
  }
  diagnostics.forEach((diagnostic, index) => {
    const item = el('button', { class: 'diag', type: 'button' }, [
      el('div', { class: 'diag-msg' }, [messages[index] ?? renderMessage(diagnostic)]),
      el('div', { class: 'diag-meta' }, [
        el('span', { class: 'diag-rule' }, [`${diagnostic.plugin}/${diagnostic.rule}`]),
        el('span', { class: 'diag-loc' }, [
          `${diagnostic.start_line}:${diagnostic.start_column + 1}`,
        ]),
      ]),
    ]);
    item.addEventListener('click', () => {
      const { from, to } = rangeFor(editor.state.doc, diagnostic);
      editor.dispatch({
        selection: { anchor: from, head: to },
        effects: EditorView.scrollIntoView(from, { y: 'center' }),
      });
      editor.focus();
    });
    diagList.append(item);
  });
}

// ---- Controls ------------------------------------------------------------

filenameInput.addEventListener('input', () => {
  filename = filenameInput.value.trim() || 'example.js';
  view.dispatch({ effects: languageConf.reconfigure(languageExtension(filename)) });
  relint();
});

sampleSelect.addEventListener('change', () => {
  if (sampleSelect.value === '') return;
  const sample: Sample | undefined = samples[Number(sampleSelect.value)];
  sampleSelect.value = '';
  if (!sample) return;
  filename = sample.filename;
  filenameInput.value = filename;
  view.dispatch({
    changes: { from: 0, to: view.state.doc.length, insert: sample.code },
    effects: languageConf.reconfigure(languageExtension(filename)),
  });
  relint();
});

let searchTimer = 0;
searchInput.addEventListener('input', () => {
  search = searchInput.value;
  window.clearTimeout(searchTimer);
  searchTimer = window.setTimeout(renderTree, 120);
});

// ---- First paint ---------------------------------------------------------

renderTree();
