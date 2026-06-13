    use super::{is_hook_name, is_react_component_name, scan_react_hooks};

    fn message_ids(source_text: &str) -> oxlint_plugins_carton::SmallVec<[&'static str; 16]> {
        scan_react_hooks(source_text, "Component.tsx")
            .into_iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect()
    }

    #[test]
    fn classifies_component_and_hook_names() {
        let cases = [
            ("Component", true, false),
            ("CMS", true, false),
            ("useState", false, true),
            ("use2", false, true),
            ("use", false, true),
            ("use_state", false, false),
            ("component", false, false),
        ];

        for (name, component, hook) in cases {
            assert_eq!(is_react_component_name(name), component);
            assert_eq!(is_hook_name(name), hook);
        }
    }

    #[test]
    fn scans_rules_of_hooks_categories() {
        let cases = [
            ("useState();\n", &["topLevel"][..]),
            (
                "function normal() { useState(); }\n",
                &["invalidFunction"][..],
            ),
            (
                "function Component() { items.map(() => { useState(); }); }\n",
                &["callback"][..],
            ),
            (
                "function Component() { if (cond) { useState(); } }\n",
                &["conditional"][..],
            ),
            (
                "function Component() { if (cond) return null; useState(); }\n",
                &["conditional"][..],
            ),
            (
                "function Component() { while (cond) { useState(); } }\n",
                &["loop"][..],
            ),
            (
                "async function Component() { useState(); }\n",
                &["async"][..],
            ),
            (
                "class App extends React.Component { render() { useState(); } }\n",
                &["class"][..],
            ),
            (
                "function Component() { try { use(resource); } catch (error) {} }\n",
                &["tryCatch"][..],
            ),
            (
                "function Component() { if (cond) { use(resource); } }\n",
                &[][..],
            ),
        ];

        for (source_text, expected) in cases {
            assert_eq!(message_ids(source_text).as_slice(), expected);
        }
    }
