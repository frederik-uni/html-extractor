use std::{cell::RefCell, rc::Rc};

use indexmap::{IndexMap, IndexSet};

use crate::{Object, Value, Visibility};

#[derive(Clone, Debug, Default)]
pub struct Inputs(IndexMap<String, Binding>);

impl Inputs {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn insert(mut self, name: impl Into<String>, value: Value, visibility: Visibility) -> Self {
        self.0.insert(name.into(), Binding { value, visibility });
        self
    }

    pub(crate) fn get(&self, name: &str) -> Option<&Value> {
        self.0.get(name).map(|binding| &binding.value)
    }
}

#[derive(Clone, Debug)]
struct Binding {
    value: Value,
    visibility: Visibility,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Scope {
    bindings: IndexMap<String, Binding>,
    local_names: IndexSet<String>,
    globals: Rc<RefCell<IndexMap<String, Binding>>>,
    extensions: Rc<RefCell<Object>>,
    root: bool,
    function_frame: bool,
}

impl Scope {
    pub(crate) fn root(inputs: Inputs) -> Self {
        Self {
            bindings: IndexMap::new(),
            local_names: IndexSet::new(),
            globals: Rc::new(RefCell::new(inputs.0)),
            extensions: Rc::new(RefCell::new(Object::new())),
            root: true,
            function_frame: false,
        }
    }

    pub(crate) fn child(&self) -> Self {
        Self {
            bindings: self.bindings.clone(),
            local_names: IndexSet::new(),
            globals: Rc::clone(&self.globals),
            extensions: Rc::clone(&self.extensions),
            root: false,
            function_frame: false,
        }
    }

    pub(crate) fn function(&self) -> Self {
        Self {
            bindings: IndexMap::new(),
            local_names: IndexSet::new(),
            globals: Rc::clone(&self.globals),
            extensions: Rc::clone(&self.extensions),
            root: false,
            function_frame: true,
        }
    }

    pub(crate) fn assign(&mut self, name: impl Into<String>, value: Value, visibility: Visibility) {
        let name = name.into();
        if self.root {
            self.globals
                .borrow_mut()
                .insert(name, Binding { value, visibility });
            return;
        }
        if self.function_frame
            && !self.bindings.contains_key(&name)
            && self.globals.borrow().contains_key(&name)
        {
            let visibility = self.globals.borrow()[&name].visibility;
            self.globals
                .borrow_mut()
                .insert(name, Binding { value, visibility });
            return;
        }
        self.bindings
            .insert(name.clone(), Binding { value, visibility });
        self.local_names.insert(name);
    }

    pub(crate) fn get(&self, name: &str) -> Option<Value> {
        self.bindings
            .get(name)
            .map(|binding| binding.value.clone())
            .or_else(|| {
                self.globals
                    .borrow()
                    .get(name)
                    .map(|binding| binding.value.clone())
            })
            .or_else(|| self.extensions.borrow().get(name).cloned())
    }

    pub(crate) fn extend(&mut self, object: Object) {
        let mut extensions = self.extensions.borrow_mut();
        for (name, value) in &object {
            self.bindings.insert(
                name.clone(),
                Binding {
                    value: value.clone(),
                    visibility: Visibility::Public,
                },
            );
            self.local_names.insert(name.clone());
            extensions.insert(name.clone(), value.clone());
        }
    }

    pub(crate) fn output(&self) -> Object {
        let mut output = Object::new();
        if self.root {
            for (name, binding) in &*self.globals.borrow() {
                if binding.visibility == Visibility::Public {
                    output.insert(name.clone(), binding.value.clone());
                }
            }
        }
        for (name, binding) in &self.bindings {
            let belongs = self.local_names.contains(name);
            if belongs && binding.visibility == Visibility::Public {
                output.insert(name.clone(), binding.value.clone());
            }
        }
        for (name, value) in &*self.extensions.borrow() {
            output.insert(name.clone(), value.clone());
        }
        output
    }
}
