use std::collections::{HashMap, HashSet};

use mlua::{Lua, LuaOptions, MultiValue, StdLib, Table, Value as LuaValue};

use crate::{ExecutionError, Object, Value};

#[derive(Clone, Debug)]
pub(crate) struct LuaModule {
    pub name: String,
    pub source: String,
    pub exports: Vec<String>,
}

impl LuaModule {
    pub(crate) fn load(name: String, source: String) -> Result<Self, ExecutionError> {
        if !valid_identifier(&name) {
            return Err(ExecutionError::new(
                format!("invalid Lua module name `{name}`"),
                None,
            ));
        }
        let lua = sandbox()?;
        let exports: Table = lua
            .load(&source)
            .set_name(&name)
            .eval()
            .map_err(|error| module_error(&name, error))?;
        let mut names = Vec::new();
        for pair in exports.pairs::<LuaValue, LuaValue>() {
            let (key, value) = pair.map_err(|error| module_error(&name, error))?;
            let LuaValue::String(key) = key else {
                return Err(ExecutionError::new(
                    format!("Lua module `{name}` export names must be strings"),
                    None,
                ));
            };
            let export = key
                .to_str()
                .map_err(|error| module_error(&name, error))?
                .to_owned();
            if !valid_identifier(&export) || !matches!(value, LuaValue::Function(_)) {
                return Err(ExecutionError::new(
                    format!("Lua module `{name}` export `{export}` must be a named function"),
                    None,
                ));
            }
            names.push(export);
        }
        Ok(Self {
            name,
            source,
            exports: names,
        })
    }

    pub(crate) fn call(
        &self,
        export: &str,
        arguments: Vec<Value>,
    ) -> Result<Value, ExecutionError> {
        let lua = sandbox()?;
        let exports: Table = lua
            .load(&self.source)
            .set_name(&self.name)
            .eval()
            .map_err(|error| module_error(&self.name, error))?;
        let function: mlua::Function = exports
            .get(export)
            .map_err(|error| module_error(&self.name, error))?;
        let arguments = arguments
            .iter()
            .map(|value| to_lua(&lua, value))
            .collect::<Result<Vec<_>, _>>()?;
        let value: LuaValue = function
            .call(MultiValue::from_vec(arguments))
            .map_err(|error| module_error(&self.name, error))?;
        from_lua(value, &mut HashSet::new(), 0)
    }
}

fn sandbox() -> Result<Lua, ExecutionError> {
    Lua::new_with(
        StdLib::TABLE | StdLib::STRING | StdLib::MATH | StdLib::UTF8,
        LuaOptions::default(),
    )
    .map_err(|error| ExecutionError::new(format!("failed creating Lua sandbox: {error}"), None))
}

fn to_lua(lua: &Lua, value: &Value) -> Result<LuaValue, ExecutionError> {
    Ok(match value {
        Value::Null => LuaValue::Nil,
        Value::Boolean(value) => LuaValue::Boolean(*value),
        Value::Number(value) => value.as_i64().map_or_else(
            || LuaValue::Number(value.as_f64().unwrap()),
            LuaValue::Integer,
        ),
        Value::String(value) => LuaValue::String(lua.create_string(value).map_err(lua_error)?),
        Value::Array(values) => {
            let table = lua.create_table().map_err(lua_error)?;
            for (index, value) in values.iter().enumerate() {
                table
                    .set(index + 1, to_lua(lua, value)?)
                    .map_err(lua_error)?;
            }
            LuaValue::Table(table)
        }
        Value::Object(values) => {
            let table = lua.create_table().map_err(lua_error)?;
            for (name, value) in values {
                table
                    .set(name.as_str(), to_lua(lua, value)?)
                    .map_err(lua_error)?;
            }
            LuaValue::Table(table)
        }
        value => {
            return Err(ExecutionError::new(
                format!("{} cannot cross the Lua boundary yet", value.kind()),
                None,
            ));
        }
    })
}

fn from_lua(
    value: LuaValue,
    seen: &mut HashSet<usize>,
    depth: usize,
) -> Result<Value, ExecutionError> {
    if depth > 128 {
        return Err(ExecutionError::new(
            "Lua value nesting limit exceeded",
            None,
        ));
    }
    Ok(match value {
        LuaValue::Nil => Value::Null,
        LuaValue::Boolean(value) => Value::Boolean(value),
        LuaValue::Integer(value) => Value::from(value),
        LuaValue::Number(value) => Value::Number(
            serde_json::Number::from_f64(value)
                .ok_or_else(|| ExecutionError::new("Lua returned a non-finite number", None))?,
        ),
        LuaValue::String(value) => Value::from(value.to_str().map_err(lua_error)?.to_owned()),
        LuaValue::Table(table) => table_to_value(table, seen, depth + 1)?,
        _ => return Err(ExecutionError::new("unsupported Lua return value", None)),
    })
}

fn table_to_value(
    table: Table,
    seen: &mut HashSet<usize>,
    depth: usize,
) -> Result<Value, ExecutionError> {
    let pointer = table.to_pointer() as usize;
    if !seen.insert(pointer) {
        return Err(ExecutionError::new("cyclic Lua table", None));
    }
    let mut integers = HashMap::new();
    let mut object = Object::new();
    let mut has_integer = false;
    let mut has_string = false;
    for pair in table.pairs::<LuaValue, LuaValue>() {
        let (key, value) = pair.map_err(lua_error)?;
        match key {
            LuaValue::Integer(index) if index > 0 => {
                has_integer = true;
                integers.insert(index as usize, from_lua(value, seen, depth)?);
            }
            LuaValue::String(name) => {
                has_string = true;
                object.insert(
                    name.to_str().map_err(lua_error)?.to_owned(),
                    from_lua(value, seen, depth)?,
                );
            }
            _ => {
                return Err(ExecutionError::new(
                    "Lua tables must have dense integer or string keys",
                    None,
                ));
            }
        }
    }
    seen.remove(&pointer);
    if has_integer && has_string {
        return Err(ExecutionError::new("mixed Lua table is ambiguous", None));
    }
    if has_integer {
        let len = integers.len();
        let mut values = Vec::with_capacity(len);
        for index in 1..=len {
            values.push(
                integers
                    .remove(&index)
                    .ok_or_else(|| ExecutionError::new("sparse Lua table is ambiguous", None))?,
            );
        }
        Ok(Value::Array(values))
    } else {
        Ok(Value::Object(object))
    }
}

fn valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn module_error(name: &str, error: mlua::Error) -> ExecutionError {
    ExecutionError::new(format!("Lua module `{name}` failed: {error}"), None)
}
fn lua_error(error: impl std::fmt::Display) -> ExecutionError {
    ExecutionError::new(format!("Lua conversion failed: {error}"), None)
}
