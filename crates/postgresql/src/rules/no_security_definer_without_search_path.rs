//! Port of `no-security-definer-without-search-path`: disallow
//! `SECURITY DEFINER` functions that do not pin `search_path`.
//!
//! libpg_query lists function attributes as `DefElem` options: `SECURITY
//! DEFINER` is `defname = "security"` with a Boolean `arg.boolval = true`, and a
//! `SET ...` clause is `defname = "set"`.

use serde_json::Value;

use crate::RuleContext;
use crate::ast::{array_field, is_type, str_field};

pub fn run(node: &Value, _ancestors: &[&Value], ctx: &mut RuleContext) {
    if !is_type(node, "CreateFunctionStmt") {
        return;
    }
    let mut is_definer = false;
    let mut has_set = false;
    if let Some(options) = array_field(node, "options") {
        for opt in options {
            match str_field(opt, "defname") {
                Some("security") => {
                    if opt
                        .get("arg")
                        .and_then(|a| a.get("boolval"))
                        .and_then(Value::as_bool)
                        == Some(true)
                    {
                        is_definer = true;
                    }
                }
                Some("set") => has_set = true,
                _ => {}
            }
        }
    }
    if is_definer && !has_set {
        ctx.report(node, "missingSearchPath");
    }
}
