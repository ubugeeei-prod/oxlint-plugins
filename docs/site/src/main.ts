import init, {
  WasmParserOptions,
  parseAndRender,
  version as oxContentVersion,
} from '@ox-content/wasm';

import { pages } from './content.js';
import './styles.css';

const app = getAppRoot();

const parserOptions = new WasmParserOptions();
parserOptions.gfm = true;
parserOptions.tables = true;
parserOptions.taskLists = true;

await init();

render();
window.addEventListener('hashchange', render);

function render() {
  const activeId = getActiveId();
  const activePage = pages.find((page) => page.id === activeId) ?? pages[0];
  const rendered = parseAndRender(activePage.source(), parserOptions);

  app.innerHTML = `
    <div class="shell">
      <aside class="sidebar">
        <a class="brand" href="#overview" aria-label="oxlint-plugins overview">
          <span class="brand-mark">ox</span>
          <span>
            <strong>oxlint-plugins</strong>
            <small>Rust-backed rule ports</small>
          </span>
        </a>
        <nav class="nav" aria-label="Documentation">
          ${pages
            .map(
              (page) => `
                <a class="${page.id === activePage.id ? 'active' : ''}" href="#${page.id}">
                  ${page.title}
                </a>
              `,
            )
            .join('')}
        </nav>
        <div class="runtime">
          <span>ox-content</span>
          <code>${escapeHtml(oxContentVersion())}</code>
        </div>
      </aside>
      <main class="content" tabindex="-1">
        ${rendered.html}
      </main>
    </div>
  `;
}

function getActiveId() {
  return window.location.hash.replace(/^#/, '') || pages[0].id;
}

function getAppRoot() {
  const root = document.querySelector('#app');
  if (!(root instanceof HTMLElement)) {
    throw new Error('Missing #app root.');
  }

  return root;
}

function escapeHtml(value: string) {
  return value.replace(/[&<>"']/g, (char) => {
    switch (char) {
      case '&':
        return '&amp;';
      case '<':
        return '&lt;';
      case '>':
        return '&gt;';
      case '"':
        return '&quot;';
      default:
        return '&#39;';
    }
  });
}
