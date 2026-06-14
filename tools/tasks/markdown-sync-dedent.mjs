// Verbatim reimplementation of dedent v1.7.1 (MIT, https://github.com/dmnd/dedent),
// used only by tools/tasks/sync-eslint-markdown-tests.ts: the shallow upstream
// submodule does not install dev dependencies, so the sync registers this module
// in place of the real `dedent` package when importing the upstream test files.
//
// The upstream @eslint/markdown tests author multi-line Markdown with the
// `dedent` tagged template, so the captured `code`/`output` fixture strings must
// match the real package byte-for-byte. The algorithm below mirrors the published
// source exactly (the Bun-only Unicode workaround is intentionally omitted — it
// never runs under Node). For tagged-template callers `escapeSpecialCharacters`
// defaults to `Array.isArray(strings)` (true), enabling the escaped-newline,
// escaped-backtick/dollar/brace handling and the trailing `\n` -> newline pass.

function createDedent(options) {
  dedent.withOptions = (newOptions) => createDedent({ ...options, ...newOptions });
  return dedent;

  function dedent(strings, ...values) {
    const raw = typeof strings === 'string' ? [strings] : strings.raw;
    const { escapeSpecialCharacters = Array.isArray(strings), trimWhitespace = true } = options;

    // 1. Interpolate, optionally unescaping special characters.
    let result = '';
    for (let i = 0; i < raw.length; i++) {
      let next = raw[i];
      if (escapeSpecialCharacters) {
        next = next
          .replace(/\\\n[ \t]*/g, '')
          .replace(/\\`/g, '`')
          .replace(/\\\$/g, '$')
          .replace(/\\\{/g, '{');
      }
      result += next;
      if (i < values.length) {
        result += values[i];
      }
    }

    // 2. Find the minimum indentation across all non-blank lines.
    const lines = result.split('\n');
    let mindent = null;
    for (const l of lines) {
      const m = l.match(/^(\s+)\S+/);
      if (m) {
        const indent = m[1].length;
        if (!mindent) {
          mindent = indent;
        } else {
          mindent = Math.min(mindent, indent);
        }
      }
    }

    // 3. Strip the common indentation from every indented line.
    if (mindent !== null) {
      const m = mindent;
      result = lines.map((l) => (l[0] === ' ' || l[0] === '\t' ? l.slice(m) : l)).join('\n');
    }

    // 4. Trim surrounding whitespace, then restore escaped newlines.
    if (trimWhitespace) {
      result = result.trim();
    }
    if (escapeSpecialCharacters) {
      result = result.replace(/\\n/g, '\n');
    }

    return result;
  }
}

const dedent = createDedent({});
export default dedent;
