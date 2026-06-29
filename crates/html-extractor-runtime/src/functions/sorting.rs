use std::cmp::Ordering;

use html_extractor_core::{Expression, ExpressionKind, Span, Visibility};

use crate::{
    ExecutionError, Value,
    evaluator::{EvalResult, Evaluator, Flow},
    scope::Scope,
};

pub(crate) fn sorted(
    receiver: Value,
    arguments: Vec<Value>,
    span: Span,
) -> Result<Value, ExecutionError> {
    if !arguments.is_empty() {
        return Err(ExecutionError::new(
            "`sorted` expects no arguments",
            Some(span),
        ));
    }
    let mut values = receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    let keys = comparable_keys(&values, span)?;
    let mut pairs: Vec<_> = values.drain(..).zip(keys).collect();
    pairs.sort_by(|(_, left), (_, right)| left.compare(right));
    Ok(Value::Array(
        pairs.into_iter().map(|(value, _)| value).collect(),
    ))
}

pub(crate) async fn sort_by_key(
    evaluator: &mut Evaluator<'_>,
    receiver: Value,
    arguments: &[Expression],
    scope: &mut Scope,
    span: Span,
) -> EvalResult {
    let (parameter, body) = lambda(arguments, 1, "sort_by_key", span)?;
    let values = receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;
    let mut keys = Vec::with_capacity(values.len());
    for value in &values {
        let mut child = scope.child();
        child.assign(&parameter[0], value.clone(), Visibility::Private);
        match evaluator.expression(body, &mut child).await? {
            Flow::Value(value) => keys.push(value),
            flow => return Ok(flow),
        }
    }
    let keys = comparable_keys(&keys, span).map_err(|_| {
        ExecutionError::new(
            "`sort_by_key` expects each item to produce a homogeneous non-null string or number key",
            Some(span),
        )
    })?;
    let mut pairs: Vec<_> = values.into_iter().zip(keys).collect();
    pairs.sort_by(|(_, left), (_, right)| left.compare(right));
    Ok(Flow::Value(Value::Array(
        pairs.into_iter().map(|(value, _)| value).collect(),
    )))
}

pub(crate) async fn sort_by(
    evaluator: &mut Evaluator<'_>,
    receiver: Value,
    arguments: &[Expression],
    scope: &mut Scope,
    span: Span,
) -> EvalResult {
    let (parameters, body) = lambda(arguments, 2, "sort_by", span)?;
    let mut values = receiver
        .into_array()
        .map_err(|error| ExecutionError::new(error.to_string(), Some(span)))?;

    for index in 1..values.len() {
        let mut current = index;
        while current > 0 {
            let mut child = scope.child();
            child.assign(
                &parameters[0],
                values[current - 1].clone(),
                Visibility::Private,
            );
            child.assign(&parameters[1], values[current].clone(), Visibility::Private);
            let result = match evaluator.expression(body, &mut child).await? {
                Flow::Value(value) => value,
                flow => return Ok(flow),
            };
            let order = result
                .expect_number()
                .ok()
                .and_then(serde_json::Number::as_f64)
                .ok_or_else(|| {
                    ExecutionError::new("`sort_by` comparator must return a number", Some(span))
                })?;
            if order <= 0.0 {
                break;
            }
            values.swap(current - 1, current);
            current -= 1;
        }
    }
    Ok(Flow::Value(Value::Array(values)))
}

fn lambda<'a>(
    arguments: &'a [Expression],
    parameter_count: usize,
    name: &str,
    span: Span,
) -> Result<(&'a [String], &'a Expression), ExecutionError> {
    let [lambda] = arguments else {
        return Err(ExecutionError::new(
            format!("`{name}` expects one lambda"),
            Some(span),
        ));
    };
    let ExpressionKind::Lambda { parameters, body } = lambda.kind() else {
        return Err(ExecutionError::new(
            format!("`{name}` expects a lambda"),
            Some(span),
        ));
    };
    if parameters.len() != parameter_count {
        return Err(ExecutionError::new(
            format!("`{name}` expects a {parameter_count}-parameter lambda"),
            Some(span),
        ));
    }
    Ok((parameters, body))
}

fn comparable_keys(values: &[Value], span: Span) -> Result<Vec<Comparable>, ExecutionError> {
    let keys = values
        .iter()
        .map(|value| Comparable::from_value(value, span))
        .collect::<Result<Vec<_>, _>>()?;
    if let Some(first) = keys.first()
        && keys.iter().any(|key| key.kind() != first.kind())
    {
        return Err(comparable_error(span));
    }
    Ok(keys)
}

fn comparable_error(span: Span) -> ExecutionError {
    ExecutionError::new(
        "sorting expects homogeneous non-null strings or numbers",
        Some(span),
    )
}

enum Comparable {
    Number(f64),
    String(String),
}

impl Comparable {
    fn from_value(value: &Value, span: Span) -> Result<Self, ExecutionError> {
        match value {
            Value::Number(number) => number
                .as_f64()
                .map(Self::Number)
                .ok_or_else(|| comparable_error(span)),
            Value::String(value) => Ok(Self::String(value.clone())),
            _ => Err(comparable_error(span)),
        }
    }

    const fn kind(&self) -> u8 {
        match self {
            Self::Number(_) => 0,
            Self::String(_) => 1,
        }
    }

    fn compare(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left.total_cmp(right),
            (Self::String(left), Self::String(right)) => left.cmp(right),
            _ => unreachable!("comparable key kinds are validated before sorting"),
        }
    }
}
