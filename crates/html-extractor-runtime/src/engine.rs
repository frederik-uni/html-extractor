use std::{collections::HashMap, future::Future, sync::Arc};

use futures::future::BoxFuture;
use html_extractor_core::{CompiledProgram, FunctionIdentity, InputConstraint, TestBlock};

use crate::lua::LuaModule;
use crate::{
    Diagnostic, ExecutionError, HttpClient, HttpPolicy, Inputs, Object, ReqwestHttpClient,
    SerializationError, Value, evaluator::Evaluator, scope::Scope,
};

pub(crate) type HostFunction =
    Arc<dyn Fn(Vec<Value>) -> BoxFuture<'static, Result<Value, ExecutionError>> + Send + Sync>;

#[derive(Clone)]
pub struct Engine {
    host_functions: HashMap<String, HostFunction>,
    http_client: Arc<dyn HttpClient>,
    http_policy: HttpPolicy,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            host_functions: HashMap::new(),
            http_client: Arc::new(ReqwestHttpClient::default()),
            http_policy: HttpPolicy::default(),
        }
    }
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Engine")
            .field("host_functions", &self.host_functions.keys())
            .field("http_client", &self.http_client)
            .field("http_policy", &self.http_policy)
            .finish()
    }
}

impl Engine {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn builder() -> EngineBuilder {
        EngineBuilder::default()
    }

    pub async fn execute(
        &self,
        program: &CompiledProgram,
        inputs: Inputs,
    ) -> Result<ExecutionResult, ExecutionError> {
        for requirement in program.requirements().inputs() {
            let Some(value) = inputs.get(requirement.name()) else {
                return Err(ExecutionError::new(
                    format!("missing required input `{}`", requirement.name()),
                    requirement.uses().first().copied(),
                ));
            };
            let compatible = match requirement.constraint() {
                InputConstraint::Any => true,
                InputConstraint::HtmlLike => is_html_like(value),
                InputConstraint::NumberLike => matches!(value, Value::Number(_)),
            };
            if !compatible {
                let expected = match requirement.constraint() {
                    InputConstraint::Any => unreachable!(),
                    InputConstraint::HtmlLike => "an HTML-like value",
                    InputConstraint::NumberLike => "a number-like value",
                };
                return Err(ExecutionError::new(
                    format!(
                        "input `{}` requires {expected}, got {}",
                        requirement.name(),
                        value.kind()
                    ),
                    requirement.uses().first().copied(),
                ));
            }
        }
        for requirement in program.requirements().functions() {
            let key = function_key(requirement.identity());
            if !self.host_functions.contains_key(&key) {
                return Err(ExecutionError::new(
                    format!("missing required function `{key}`"),
                    requirement.uses().first().copied(),
                ));
            }
        }
        let mut scope = Scope::root(inputs);
        let mut evaluator = Evaluator::new(
            &self.host_functions,
            self.http_client.as_ref(),
            &self.http_policy,
            program.functions(),
        );
        evaluator
            .statements(program.statements(), &mut scope)
            .await?;
        Ok(ExecutionResult {
            output: scope.output(),
            warnings: evaluator.into_warnings(),
        })
    }

    pub async fn test(&self, program: &CompiledProgram) -> Result<TestReport, ExecutionError> {
        let mut passed = 0usize;
        for test in program.tests() {
            self.run_test(program, test).await?;
            passed += 1;
        }
        Ok(TestReport {
            total: program.tests().len(),
            passed,
        })
    }

    async fn run_test(
        &self,
        program: &CompiledProgram,
        test: &TestBlock,
    ) -> Result<(), ExecutionError> {
        let mut scope = Scope::root(Inputs::new());
        let mut evaluator = Evaluator::new(
            &self.host_functions,
            self.http_client.as_ref(),
            &self.http_policy,
            program.functions(),
        );

        for input in test.inputs() {
            let value = match evaluator.expression(input.value(), &mut scope).await? {
                crate::evaluator::Flow::Value(value) => value,
                crate::evaluator::Flow::Return(_) => {
                    return Err(ExecutionError::new(
                        "`return` escaped while evaluating test input",
                        Some(input.span()),
                    ));
                }
                crate::evaluator::Flow::Break => {
                    return Err(ExecutionError::new(
                        "`break` escaped while evaluating test input",
                        Some(input.span()),
                    ));
                }
            };
            scope.assign(input.name(), value, input.visibility());
        }

        if test.runs_script() {
            evaluator
                .statements(program.statements(), &mut scope)
                .await?;
            scope.assign(
                "it",
                Value::Object(scope.output()),
                crate::Visibility::Private,
            );
        }

        evaluator.statements(test.body(), &mut scope).await
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TestReport {
    total: usize,
    passed: usize,
}

impl TestReport {
    #[must_use]
    pub const fn total(&self) -> usize {
        self.total
    }

    #[must_use]
    pub const fn passed(&self) -> usize {
        self.passed
    }
}

#[derive(Default)]
pub struct EngineBuilder {
    host_functions: HashMap<String, HostFunction>,
    http_client: Option<Arc<dyn HttpClient>>,
    http_policy: HttpPolicy,
    error: Option<ExecutionError>,
    lua_modules: Vec<(String, String)>,
}

impl EngineBuilder {
    #[must_use]
    pub fn host_function<F, Fut>(mut self, name: impl Into<String>, function: F) -> Self
    where
        F: Fn(Vec<Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Value, ExecutionError>> + Send + 'static,
    {
        let name = name.into();
        if is_reserved(&name) {
            self.error = Some(ExecutionError::new(
                format!("`{name}` is a reserved core function"),
                None,
            ));
        } else {
            self.host_functions.insert(
                name,
                Arc::new(move |arguments| Box::pin(function(arguments))),
            );
        }
        self
    }

    #[must_use]
    pub fn http_client(mut self, client: Arc<dyn HttpClient>) -> Self {
        self.http_client = Some(client);
        self
    }

    #[must_use]
    pub fn http_policy(mut self, policy: HttpPolicy) -> Self {
        self.http_policy = policy;
        self
    }

    #[must_use]
    pub fn lua_module(mut self, name: impl Into<String>, source: impl Into<String>) -> Self {
        self.lua_modules.push((name.into(), source.into()));
        self
    }

    pub fn build(mut self) -> Result<Engine, ExecutionError> {
        if let Some(error) = self.error {
            return Err(error);
        }
        let mut modules = Vec::new();
        let mut module_names = std::collections::HashSet::new();
        for (name, source) in self.lua_modules {
            if !module_names.insert(name.clone()) {
                return Err(ExecutionError::new(
                    format!("duplicate Lua module `{name}`"),
                    None,
                ));
            }
            modules.push(LuaModule::load(name, source)?);
        }
        let mut counts = HashMap::<String, usize>::new();
        for module in &modules {
            for export in &module.exports {
                *counts.entry(export.clone()).or_default() += 1;
            }
        }
        for module in modules {
            for export in module.exports.clone() {
                let qualified_module = module.clone();
                let qualified_export = export.clone();
                self.host_functions.insert(
                    format!("{}.{}", module.name, export),
                    Arc::new(move |arguments| {
                        Box::pin(futures::future::ready(
                            qualified_module.call(&qualified_export, arguments),
                        ))
                    }),
                );
                if counts[&export] == 1 {
                    let unqualified_module = module.clone();
                    let unqualified_export = export.clone();
                    self.host_functions.insert(
                        export,
                        Arc::new(move |arguments| {
                            Box::pin(futures::future::ready(
                                unqualified_module.call(&unqualified_export, arguments),
                            ))
                        }),
                    );
                }
            }
        }
        Ok(Engine {
            host_functions: self.host_functions,
            http_client: self
                .http_client
                .unwrap_or_else(|| Arc::new(ReqwestHttpClient::default())),
            http_policy: self.http_policy,
        })
    }
}

fn is_reserved(name: &str) -> bool {
    matches!(name, "fetch")
}

fn function_key(identity: &FunctionIdentity) -> String {
    match identity {
        FunctionIdentity::Unqualified(name) => name.clone(),
        FunctionIdentity::Qualified { module, name } => format!("{module}.{name}"),
    }
}

fn is_html_like(value: &Value) -> bool {
    match value {
        Value::HtmlDocument(_) | Value::HtmlNode(_) | Value::Response(_) => true,
        Value::Array(values) => values.iter().all(is_html_like),
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionResult {
    output: Object,
    warnings: Vec<Diagnostic>,
}

impl ExecutionResult {
    #[must_use]
    pub const fn output(&self) -> &Object {
        &self.output
    }

    #[must_use]
    pub fn warnings(&self) -> &[Diagnostic] {
        &self.warnings
    }

    pub fn to_json(&self) -> Result<serde_json::Value, SerializationError> {
        Value::Object(self.output.clone()).to_json()
    }
}
