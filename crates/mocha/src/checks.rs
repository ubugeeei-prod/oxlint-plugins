//! Per-entity rule checks (suites, test cases, hooks, callbacks, titles) for
//! the mocha scanner.

use oxlint_plugins_carton::CompactString;

use crate::helpers::{
    callback_body_calls_identifier, callback_body_contains_this, callback_body_returns_promise,
    callback_body_returns_value, compact_format, display_call_name,
};
use crate::scanner::Scanner;
use crate::{Callback, Entity, EntityType, Modifier};

impl<'a> Scanner<'a> {
    pub(crate) fn handle_entity(&mut self, entity: &Entity<'a>) {
        self.has_test_entity = true;
        if entity.interface.as_str() != self.options.consistent_interface.as_str() {
            self.report(
                "consistent-interface",
                compact_format(format_args!(
                    "Unexpected use of {} interface instead of {}",
                    entity.interface.as_str(),
                    self.options.consistent_interface
                )),
                entity.span,
            );
        }

        if entity.entity_type == EntityType::Suite && self.suite_depth == 0 {
            self.top_level_suites += 1;
            if self.top_level_suites == self.options.max_top_level_suites_limit + 1 {
                self.report(
                    "max-top-level-suites",
                    compact_format(format_args!(
                        "The number of top-level suites is more than {}.",
                        self.options.max_top_level_suites_limit
                    )),
                    entity.span,
                );
            }
        }

        match entity.entity_type {
            EntityType::Suite => self.check_suite(entity),
            EntityType::TestCase => self.check_test_case(entity),
            EntityType::Hook => self.check_hook(entity),
        }

        if let Some(callback) = entity.callback {
            self.check_callback(entity, callback);
        } else if entity.entity_type == EntityType::TestCase
            && entity.modifier != Some(Modifier::Pending)
        {
            self.report(
                "no-pending-tests",
                "Unexpected pending mocha test.",
                entity.span,
            );
        }
    }

    fn check_suite(&mut self, entity: &Entity<'a>) {
        if entity.modifier == Some(Modifier::Exclusive) {
            self.report(
                "no-exclusive-tests",
                "Unexpected exclusive mocha test.",
                entity.span,
            );
        }
        if entity.modifier == Some(Modifier::Pending) {
            self.report(
                "no-pending-tests",
                "Unexpected pending mocha test.",
                entity.span,
            );
        }
        let invalid_suite_title = self.valid_suite_regex.as_ref().is_some_and(|pattern| {
            entity
                .title
                .as_ref()
                .is_some_and(|title| !pattern.is_match(title.as_str()))
        });
        self.check_title(
            "valid-suite-title",
            entity,
            invalid_suite_title,
            self.options.valid_suite_title_message.clone(),
            "Invalid \"",
        );
        self.check_empty_title(entity);
        if let Some(title) = &entity.title {
            let current = self.layers.last_mut().expect("root layer exists");
            if current.suite_titles.contains_key(title.as_str()) {
                self.report(
                    "no-identical-title",
                    compact_format(format_args!(
                        "Unexpected use of duplicate Mocha title `{title}`"
                    )),
                    entity.span,
                );
            } else {
                current.suite_titles.insert(title.clone(), entity.span);
            }
        }
    }

    fn check_test_case(&mut self, entity: &Entity<'a>) {
        if self.test_depth > 0 {
            self.report(
                "no-nested-tests",
                "Unexpected test nested inside another test.",
                entity.span,
            );
        }
        if self.suite_depth == 0 {
            self.report(
                "no-global-tests",
                "Unexpected global mocha test.",
                entity.span,
            );
        }
        if entity.modifier == Some(Modifier::Exclusive) {
            self.report(
                "no-exclusive-tests",
                "Unexpected exclusive mocha test.",
                entity.span,
            );
        }
        if entity.modifier == Some(Modifier::Pending) {
            self.report(
                "no-pending-tests",
                "Unexpected pending mocha test.",
                entity.span,
            );
        }
        let invalid_test_title = self.valid_test_regex.as_ref().is_some_and(|pattern| {
            entity
                .title
                .as_ref()
                .is_some_and(|title| !pattern.is_match(title.as_str()))
        });
        self.check_title(
            "valid-test-title",
            entity,
            invalid_test_title,
            self.options.valid_test_title_message.clone(),
            "Invalid \"",
        );
        self.check_empty_title(entity);
        let current = self.layers.last_mut().expect("root layer exists");
        current.test_count += 1;
        if let Some(title) = &entity.title {
            if current.test_titles.contains_key(title.as_str()) {
                self.report(
                    "no-identical-title",
                    compact_format(format_args!(
                        "Unexpected use of duplicate Mocha title `{title}`"
                    )),
                    entity.span,
                );
            } else {
                current.test_titles.insert(title.clone(), entity.span);
            }
        }
    }

    fn check_hook(&mut self, entity: &Entity<'a>) {
        if self.suite_depth == 0 {
            self.report(
                "no-top-level-hooks",
                compact_format(format_args!(
                    "Unexpected use of Mocha `{}` hook outside of a test suite",
                    display_call_name(entity.name.as_str())
                )),
                entity.span,
            );
        }
        if !self
            .options
            .no_hooks_allowed
            .iter()
            .any(|allowed| allowed.as_str() == entity.name.as_str())
        {
            self.report(
                "no-hooks",
                compact_format(format_args!(
                    "Unexpected use of Mocha `{}` hook",
                    display_call_name(entity.name.as_str())
                )),
                entity.span,
            );
        }
        let current = self.layers.last_mut().expect("root layer exists");
        current.hooks.push((entity.name.clone(), entity.span));
        if current.hook_names.contains_key(entity.name.as_str()) {
            self.report(
                "no-sibling-hooks",
                compact_format(format_args!(
                    "Unexpected use of duplicate Mocha `{}` hook",
                    display_call_name(entity.name.as_str())
                )),
                entity.span,
            );
        } else {
            current.hook_names.insert(entity.name.clone(), entity.span);
        }
    }

    fn check_callback(&mut self, entity: &Entity<'a>, callback: Callback<'a>) {
        if entity.entity_type == EntityType::Suite && callback.async_function {
            self.report(
                "no-async-suite",
                compact_format(format_args!(
                    "Unexpected async function in {}",
                    display_call_name(entity.name.as_str())
                )),
                callback.span,
            );
        }
        if callback.arrow {
            self.report(
                "no-mocha-arrows",
                "Unexpected arrow function.",
                callback.span,
            );
        } else if !self.callback_is_allowed_function_callback(callback) {
            self.report(
                "prefer-arrow-callback",
                "Unexpected function expression.",
                callback.span,
            );
        }
        if matches!(entity.entity_type, EntityType::TestCase | EntityType::Hook) {
            if let Some(param) = callback.first_param_name
                && !(entity.modifier == Some(Modifier::Pending)
                    && self.options.handle_done_ignore_pending)
                && !callback_body_calls_identifier(callback.body, param)
            {
                self.report(
                    "handle-done-callback",
                    compact_format(format_args!("Expected \"{param}\" callback to be handled.")),
                    callback.span,
                );
            }
            if callback.first_param_name.is_some() && callback_body_returns_value(callback.body) {
                self.report(
                    "no-return-and-callback",
                    "Unexpected use of `return` in a test with callback",
                    callback.span,
                );
            }
            if callback.async_function && callback_body_returns_value(callback.body) {
                self.report(
                    "no-return-from-async",
                    "Unexpected use of `return` in a test with an async function",
                    callback.span,
                );
            }
            if self.callback_is_synchronous(callback) {
                self.report(
                    "no-synchronous-tests",
                    "Unexpected synchronous test.",
                    callback.span,
                );
            }
        }
    }

    fn callback_is_synchronous(&self, callback: Callback<'a>) -> bool {
        let mut async_used = false;
        for method in &self.options.no_synchronous_allowed {
            match method.as_str() {
                "async" if callback.async_function => async_used = true,
                "callback" if callback.params_len == 1 => async_used = true,
                "promise" if callback_body_returns_promise(callback.body) => async_used = true,
                _ => {}
            }
        }
        !async_used
    }

    fn callback_is_allowed_function_callback(&self, callback: Callback<'a>) -> bool {
        (self.options.prefer_arrow_allow_named_functions && callback.named_function)
            || (self.options.prefer_arrow_allow_unbound_this
                && callback_body_contains_this(callback.body))
    }

    fn check_empty_title(&mut self, entity: &Entity<'a>) {
        let empty = entity
            .title
            .as_ref()
            .is_none_or(|title| title.trim().is_empty());
        if empty {
            self.report(
                "no-empty-title",
                self.options
                    .no_empty_title_message
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| "Unexpected empty test description.".into()),
                entity.span,
            );
        }
    }

    fn check_title(
        &mut self,
        rule_name: &'static str,
        entity: &Entity<'a>,
        invalid_title: bool,
        custom_message: Option<CompactString>,
        default_prefix: &str,
    ) {
        if !invalid_title {
            return;
        };
        self.report(
            rule_name,
            custom_message.unwrap_or_else(|| {
                compact_format(format_args!(
                    "{default_prefix}{}\" description found.",
                    display_call_name(entity.name.as_str())
                ))
            }),
            entity.span,
        );
    }
}
