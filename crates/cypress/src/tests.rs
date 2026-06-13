    use super::{CypressOptions, scan_cypress};
    use oxlint_plugins_carton::SmallVec;

    fn rule_names(source_text: &str) -> SmallVec<[&'static str; 16]> {
        scan_cypress(source_text, "fixture.tsx", &CypressOptions::default())
            .into_iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect()
    }

    #[test]
    fn scans_core_cypress_rules() {
        let rules = rule_names(
            r#"
            const a = cy.get(".foo");
            before("x", async () => { cy.get(".foo"); });
            it("x", async () => { Cypress.env("x"); });
            cy.get(".foo").and("be.visible");
            cy.debug();
            cy.pause();
            cy.xpath("//main");
            cy.wait(100);
            cy.get(".foo").click({ force: true }).should("exist");
            cy.visit("/home");
            cy.screenshot();
            "#,
        );

        assert!(rules.contains(&"no-assigning-return-values"));
        assert!(rules.contains(&"no-async-before"));
        assert!(rules.contains(&"no-async-tests"));
        assert!(rules.contains(&"no-and"));
        assert!(rules.contains(&"no-debug"));
        assert!(rules.contains(&"no-pause"));
        assert!(rules.contains(&"no-xpath"));
        assert!(rules.contains(&"no-unnecessary-waiting"));
        assert!(rules.contains(&"no-force"));
        assert!(rules.contains(&"unsafe-to-chain-command"));
        assert!(rules.contains(&"assertion-before-screenshot"));
        assert!(rules.contains(&"require-data-selectors"));
    }

    #[test]
    fn tracks_data_selector_variables_and_wait_defaults() {
        let rules = rule_names(
            r#"
            const GOOD = "[data-cy=submit]";
            cy.get(GOOD);
            function customWait({ ms = 1 }) { cy.wait(ms); }
            "#,
        );

        assert!(!rules.contains(&"require-data-selectors"));
        assert!(rules.contains(&"no-unnecessary-waiting"));
    }

    #[test]
    fn supports_custom_unsafe_methods() {
        let mut options = CypressOptions::default();
        options.unsafe_to_chain_methods.push("customType".into());
        let rules = scan_cypress(
            r#"cy.get("new-todo").customType("todo").should("have.class", "active");"#,
            "fixture.ts",
            &options,
        )
        .into_iter()
        .map(|diagnostic| diagnostic.rule_name)
        .collect::<SmallVec<[_; 16]>>();

        assert!(rules.contains(&"unsafe-to-chain-command"));
    }
