//! NAPI boundary for the `no-forbidden-identifiers` sample plugin.
//!
//! Rule logic lives in `oxlint_plugins_stylistic`; this module only compacts the
//! JavaScript-supplied options at the boundary before the hot logic runs.

use oxlint_plugins_carton::{CompactString, SmallVec};

pub use napi_abi::{
    ForbiddenIdentifierOptions, is_forbidden_identifier_name, scan_forbidden_identifiers,
};

type OptionNames = SmallVec<[CompactString; 8]>;

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI and proc macros require String/Vec/format expansion; this module compacts values before rule logic runs."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_stylistic::{
        is_forbidden_identifier_name as core_is_forbidden_identifier_name, scan_source_for_rule,
    };

    use super::OptionNames;

    #[napi(object, namespace = "noForbiddenIdentifiers")]
    #[derive(Clone, Debug, Default)]
    pub struct ForbiddenIdentifierOptions {
        // NAPI converts JavaScript arrays and strings through `Vec<String>`.
        // Rule code must compact these values at the boundary before hot logic runs.
        pub names: Option<Vec<String>>,
    }

    #[napi(namespace = "noForbiddenIdentifiers")]
    pub fn scan_forbidden_identifiers(
        source_text: String,
        options: Option<ForbiddenIdentifierOptions>,
    ) -> Vec<String> {
        let option_names = compact_option_names(options.as_ref());
        scan_source_for_rule(&source_text, option_names.iter().map(CompactString::as_str))
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    #[napi(namespace = "noForbiddenIdentifiers")]
    pub fn is_forbidden_identifier_name(
        name: String,
        options: Option<ForbiddenIdentifierOptions>,
    ) -> bool {
        let option_names = compact_option_names(options.as_ref());
        core_is_forbidden_identifier_name(&name, option_names.iter().map(CompactString::as_str))
    }

    fn compact_option_names(options: Option<&ForbiddenIdentifierOptions>) -> OptionNames {
        let mut names = SmallVec::new();

        if let Some(option_names) = options.and_then(|options| options.names.as_deref()) {
            for name in option_names {
                if !name.is_empty() {
                    names.push(CompactString::from(name.as_str()));
                }
            }
        }

        names
    }
}
