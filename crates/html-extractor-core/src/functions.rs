#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuiltinFunctionKind {
    Core,
    Lambda,
    Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BuiltinFunction {
    name: &'static str,
    kind: BuiltinFunctionKind,
}

impl BuiltinFunction {
    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn kind(self) -> BuiltinFunctionKind {
        self.kind
    }
}

const fn builtin(name: &'static str, kind: BuiltinFunctionKind) -> BuiltinFunction {
    BuiltinFunction { name, kind }
}

pub const BUILTIN_FUNCTIONS: &[BuiltinFunction] = &[
    builtin("fetch", BuiltinFunctionKind::Core),
    builtin("extend", BuiltinFunctionKind::Core),
    builtin("map", BuiltinFunctionKind::Lambda),
    builtin("filter", BuiltinFunctionKind::Lambda),
    builtin("any", BuiltinFunctionKind::Lambda),
    builtin("all", BuiltinFunctionKind::Lambda),
    builtin("find", BuiltinFunctionKind::Lambda),
    builtin("sort_by_key", BuiltinFunctionKind::Lambda),
    builtin("sort_by", BuiltinFunctionKind::Lambda),
    builtin("assert", BuiltinFunctionKind::Lambda),
    builtin("warn", BuiltinFunctionKind::Lambda),
    builtin("css", BuiltinFunctionKind::Value),
    builtin("css@", BuiltinFunctionKind::Value),
    builtin("flatten", BuiltinFunctionKind::Value),
    builtin("text", BuiltinFunctionKind::Value),
    builtin("keys", BuiltinFunctionKind::Value),
    builtin("log", BuiltinFunctionKind::Value),
    builtin("trim", BuiltinFunctionKind::Value),
    builtin("replace", BuiltinFunctionKind::Value),
    builtin("urlencode", BuiltinFunctionKind::Value),
    builtin("contains", BuiltinFunctionKind::Value),
    builtin("starts_with", BuiltinFunctionKind::Value),
    builtin("inner_html", BuiltinFunctionKind::Value),
    builtin("html_text", BuiltinFunctionKind::Value),
    builtin("href", BuiltinFunctionKind::Value),
    builtin("len", BuiltinFunctionKind::Value),
    builtin("range", BuiltinFunctionKind::Value),
    builtin("zip", BuiltinFunctionKind::Value),
    builtin("first", BuiltinFunctionKind::Value),
    builtin("last", BuiltinFunctionKind::Value),
    builtin("take", BuiltinFunctionKind::Value),
    builtin("unique", BuiltinFunctionKind::Value),
    builtin("values", BuiltinFunctionKind::Value),
    builtin("has", BuiltinFunctionKind::Value),
    builtin("src", BuiltinFunctionKind::Value),
    builtin("unwrap_or", BuiltinFunctionKind::Value),
    builtin("attr", BuiltinFunctionKind::Value),
    builtin("html", BuiltinFunctionKind::Value),
    builtin("regex", BuiltinFunctionKind::Value),
    builtin("regex@", BuiltinFunctionKind::Value),
    builtin("sorted", BuiltinFunctionKind::Value),
    builtin("status", BuiltinFunctionKind::Value),
    builtin("json", BuiltinFunctionKind::Value),
    builtin("ensure_suffix", BuiltinFunctionKind::Value),
    builtin("join", BuiltinFunctionKind::Value),
    builtin("nullcheck", BuiltinFunctionKind::Value),
];

#[must_use]
pub fn builtin_function(name: &str) -> Option<BuiltinFunction> {
    BUILTIN_FUNCTIONS
        .iter()
        .copied()
        .find(|function| function.name == name)
}

#[cfg(test)]
mod tests {
    use super::BUILTIN_FUNCTIONS;

    #[test]
    fn learning_guide_indexes_every_builtin_function() {
        let guide = include_str!("../../../docs/learning.md");
        for function in BUILTIN_FUNCTIONS {
            let table_entry = format!("| `{}` |", function.name());
            assert!(
                guide.contains(&table_entry),
                "missing function index entry for `{}`",
                function.name()
            );
        }
    }
}
