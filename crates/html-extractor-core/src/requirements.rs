use crate::Span;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Requirements {
    inputs: Vec<InputRequirement>,
    functions: Vec<FunctionRequirement>,
}

impl Requirements {
    #[must_use]
    pub fn inputs(&self) -> &[InputRequirement] {
        &self.inputs
    }

    #[must_use]
    pub fn functions(&self) -> &[FunctionRequirement] {
        &self.functions
    }

    pub(crate) fn require_input(&mut self, name: &str, span: Span) -> usize {
        if let Some(index) = self.inputs.iter().position(|input| input.name == name) {
            self.inputs[index].uses.push(span);
            return index;
        }
        self.inputs.push(InputRequirement {
            name: name.to_owned(),
            constraint: InputConstraint::Any,
            uses: vec![span],
        });
        self.inputs.len() - 1
    }

    pub(crate) fn constrain_input(
        &mut self,
        index: usize,
        constraint: InputConstraint,
    ) -> Result<(), InputConstraint> {
        let current = self.inputs[index].constraint;
        match (current, constraint) {
            (InputConstraint::Any, next) => self.inputs[index].constraint = next,
            (left, right) if left == right || right == InputConstraint::Any => {}
            (_, right) => return Err(right),
        }
        Ok(())
    }

    pub(crate) fn require_function(
        &mut self,
        identity: FunctionIdentity,
        arity: usize,
        span: Span,
    ) {
        if let Some(function) = self
            .functions
            .iter_mut()
            .find(|function| function.identity == identity)
        {
            if !function.arities.contains(&arity) {
                function.arities.push(arity);
                function.arities.sort_unstable();
            }
            function.uses.push(span);
            return;
        }
        self.functions.push(FunctionRequirement {
            identity,
            arities: vec![arity],
            uses: vec![span],
        });
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InputRequirement {
    name: String,
    constraint: InputConstraint,
    uses: Vec<Span>,
}

impl InputRequirement {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn constraint(&self) -> InputConstraint {
        self.constraint
    }

    #[must_use]
    pub fn uses(&self) -> &[Span] {
        &self.uses
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum InputConstraint {
    #[default]
    Any,
    HtmlLike,
    NumberLike,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum FunctionIdentity {
    Unqualified(String),
    Qualified { module: String, name: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FunctionRequirement {
    identity: FunctionIdentity,
    arities: Vec<usize>,
    uses: Vec<Span>,
}

impl FunctionRequirement {
    #[must_use]
    pub const fn identity(&self) -> &FunctionIdentity {
        &self.identity
    }

    #[must_use]
    pub fn arities(&self) -> &[usize] {
        &self.arities
    }

    #[must_use]
    pub fn uses(&self) -> &[Span] {
        &self.uses
    }
}
