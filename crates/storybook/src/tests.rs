use super::*;

fn scan(rule_name: &'static str, source_text: &str) -> SmallVec<[Diagnostic; 16]> {
    let mut options = StorybookOptions::default();
    options.rule_names.clear();
    options.rule_names.push(CompactString::from(rule_name));
    scan_storybook(source_text, "Button.stories.tsx", &options)
}

#[test]
fn scans_interaction_rules() {
    let diagnostics = scan(
        "await-interactions",
        "Basic.play = async () => { userEvent.click(button) }",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].rule_name, "await-interactions");
    assert_eq!(diagnostics[0].message_id, "interactionShouldBeAwaited");
    assert_eq!(
        diagnostics[0]
            .data
            .method
            .as_ref()
            .map(CompactString::as_str),
        Some("userEvent")
    );

    let diagnostics = scan(
        "context-in-play-function",
        "export const SecondStory = { play: async ({ canvasElement }) => { await FirstStory.play({ canvasElement }) } }",
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message_id, "passContextToPlayFunction");
}

#[test]
fn scans_meta_rules() {
    assert_eq!(
        scan("csf-component", "export default { title: 'Button' }")[0].message_id,
        "missingComponentProperty"
    );
    assert_eq!(
        scan(
            "hierarchy-separator",
            "export default { title: 'Atoms|Button', component: Button }",
        )[0]
        .message_id,
        "deprecatedHierarchySeparator"
    );
    assert_eq!(
        scan(
            "meta-inline-properties",
            "const title = 'Button'; export default { title, component: Button }",
        )[0]
        .data
        .property
        .as_ref()
        .map(CompactString::as_str),
        Some("title")
    );
    assert_eq!(
        scan(
            "meta-satisfies-type",
            "const meta: Meta<typeof Button> = { component: Button }; export default meta;",
        )[0]
        .fixes
        .len(),
        2
    );
}

#[test]
fn scans_story_exports_and_story_names() {
    assert_eq!(
        scan(
            "no-redundant-story-name",
            "export const PrimaryButton = { name: 'Primary Button' }",
        )[0]
        .message_id,
        "storyNameIsRedundant"
    );
    assert_eq!(
        scan("prefer-pascal-case", "export const primary_story = {};")[0]
            .data
            .name
            .as_ref()
            .map(CompactString::as_str),
        Some("primary_story")
    );
    assert_eq!(
        scan("story-exports", "export default { component: Button }")[0].message_id,
        "shouldHaveStoryExport"
    );
}

#[test]
fn scans_imports_and_addons() {
    assert_eq!(
        scan(
            "no-renderer-packages",
            "import { Meta } from '@storybook/react'"
        )[0]
        .message_id,
        "noRendererPackages"
    );
    assert_eq!(
        scan(
            "no-stories-of",
            "import { storiesOf } from '@storybook/react'"
        )[0]
        .message_id,
        "doNotUseStoriesOf"
    );
    assert_eq!(
        scan(
            "use-storybook-testing-library",
            "import userEvent from '@testing-library/user-event'",
        )[0]
        .fixes
        .len(),
        2
    );

    let mut options = StorybookOptions::default();
    options.rule_names.clear();
    options
        .rule_names
        .push(CompactString::from("no-uninstalled-addons"));
    options
        .installed_addons
        .push(CompactString::from("@storybook/addon-essentials"));
    let diagnostics = scan_storybook(
        "export default { addons: ['@storybook/addon-essentials', '@storybook/not-installed'] }",
        "main.ts",
        &options,
    );
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0]
            .data
            .addon_name
            .as_ref()
            .map(CompactString::as_str),
        Some("@storybook/not-installed")
    );
}
