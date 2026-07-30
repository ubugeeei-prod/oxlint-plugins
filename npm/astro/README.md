# @oxlint-plugins/oxlint-plugin-astro

Rust-backed Oxlint plugin port of selected `eslint-plugin-astro` rules.

The Rust core parses frontmatter with Oxc and conservatively segments template
expressions, attributes, and element bodies:

- `no-deprecated-astro-canonicalurl`
- `no-deprecated-astro-fetchcontent`
- `no-deprecated-astro-resolve`
- `no-deprecated-getentrybyslug`
- `no-set-html-directive`
- `no-set-text-directive`
- `prefer-class-list-directive`

Oxlint 1.68 exposes `.astro` JavaScript plugins through a virtual frontmatter
source. Physical template-body diagnostics are mapped back to their real
locations; body fixes remain available through the direct API but are omitted
from the Oxlint adapter because Oxlint rejects edits outside that virtual
source. React rules are outside the Astro port's scope because Oxc handles
React rules separately.
