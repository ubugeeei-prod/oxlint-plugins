export function createTextDocument(uri, text) {
  return { uri, text };
}

export function diagnosticForIdentifier(document, ruleId, identifier, message) {
  const range = rangeForFirstMatch(document.text, identifier);

  return {
    range,
    severity: 1,
    source: 'oxlint-plugins',
    code: ruleId,
    message,
    data: {
      ruleId,
      identifier,
    },
  };
}

export function quickFixReplaceIdentifier(document, diagnostic, replacement) {
  return {
    title: `Replace ${diagnostic.data.identifier} with ${replacement}`,
    kind: 'quickfix',
    diagnostics: [diagnostic],
    edit: {
      changes: {
        [document.uri]: [
          {
            range: diagnostic.range,
            newText: replacement,
          },
        ],
      },
    },
  };
}

export function applyTextEdits(text, edits) {
  return edits
    .slice()
    .sort((a, b) => offsetForPosition(text, b.range.start) - offsetForPosition(text, a.range.start))
    .reduce((current, edit) => {
      const start = offsetForPosition(current, edit.range.start);
      const end = offsetForPosition(current, edit.range.end);
      return `${current.slice(0, start)}${edit.newText}${current.slice(end)}`;
    }, text);
}

function rangeForFirstMatch(text, needle) {
  const offset = text.indexOf(needle);
  if (offset < 0) {
    throw new Error(`Identifier ${needle} was not found in fixture text.`);
  }

  return {
    start: positionForOffset(text, offset),
    end: positionForOffset(text, offset + needle.length),
  };
}

function positionForOffset(text, offset) {
  let line = 0;
  let character = 0;

  for (let index = 0; index < offset; index += 1) {
    if (text.charCodeAt(index) === 10) {
      line += 1;
      character = 0;
    } else {
      character += 1;
    }
  }

  return { line, character };
}

function offsetForPosition(text, position) {
  let line = 0;
  let character = 0;

  for (let index = 0; index < text.length; index += 1) {
    if (line === position.line && character === position.character) {
      return index;
    }

    if (text.charCodeAt(index) === 10) {
      line += 1;
      character = 0;
    } else {
      character += 1;
    }
  }

  return text.length;
}
