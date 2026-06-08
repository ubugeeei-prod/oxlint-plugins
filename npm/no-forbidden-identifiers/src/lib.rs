use oxlint_plugins_carton::{CompactString, SmallVec};

type OptionNames = SmallVec<[CompactString; 8]>;

pub use napi_abi::{
    ForbiddenIdentifierOptions, is_forbidden_identifier_name, scan_forbidden_identifiers,
};

#[allow(
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec<String>; this module compacts values before rule logic runs."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_stylistic::{
        is_forbidden_identifier_name as core_is_forbidden_identifier_name, scan_source_for_rule,
    };

    use crate::OptionNames;

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct ForbiddenIdentifierOptions {
        // NAPI converts JavaScript arrays and strings through `Vec<String>`.
        // Rule code must compact these values at the boundary before hot logic runs.
        pub names: Option<Vec<String>>,
    }

    #[napi]
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

    #[napi]
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
