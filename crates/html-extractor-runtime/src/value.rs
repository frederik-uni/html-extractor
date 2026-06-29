use std::fmt;

use indexmap::IndexMap;

use crate::{HtmlDocument, HtmlNode, Response, SerializationError, TypeError};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Object(IndexMap<String, Value>);

impl Object {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl Into<String>, value: Value) -> Option<Value> {
        self.0.insert(name.into(), value)
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.0.get(name)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.0.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for &'a Object {
    type Item = (&'a String, &'a Value);
    type IntoIter = indexmap::map::Iter<'a, String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Boolean(bool),
    Number(serde_json::Number),
    String(String),
    Array(Vec<Value>),
    Object(Object),
    HtmlDocument(HtmlDocument),
    HtmlNode(HtmlNode),
    Response(Response),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueKind {
    Null,
    Boolean,
    Number,
    String,
    Array,
    Object,
    HtmlDocument,
    HtmlNode,
    Response,
}

impl fmt::Display for ValueKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Null => "null",
            Self::Boolean => "boolean",
            Self::Number => "number",
            Self::String => "string",
            Self::Array => "array",
            Self::Object => "object",
            Self::HtmlDocument => "HTML document",
            Self::HtmlNode => "HTML node",
            Self::Response => "HTTP response",
        })
    }
}

impl Value {
    #[must_use]
    pub const fn kind(&self) -> ValueKind {
        match self {
            Self::Null => ValueKind::Null,
            Self::Boolean(_) => ValueKind::Boolean,
            Self::Number(_) => ValueKind::Number,
            Self::String(_) => ValueKind::String,
            Self::Array(_) => ValueKind::Array,
            Self::Object(_) => ValueKind::Object,
            Self::HtmlDocument(_) => ValueKind::HtmlDocument,
            Self::HtmlNode(_) => ValueKind::HtmlNode,
            Self::Response(_) => ValueKind::Response,
        }
    }

    pub fn expect_array(&self) -> Result<&[Value], TypeError> {
        match self {
            Self::Array(values) => Ok(values),
            _ => Err(TypeError::new("array", self.kind())),
        }
    }

    pub fn into_array(self) -> Result<Vec<Value>, TypeError> {
        let kind = self.kind();
        match self {
            Self::Array(values) => Ok(values),
            _ => Err(TypeError::new("array", kind)),
        }
    }

    pub fn expect_string(&self) -> Result<&str, TypeError> {
        match self {
            Self::String(value) => Ok(value),
            _ => Err(TypeError::new("string", self.kind())),
        }
    }

    pub fn expect_boolean(&self) -> Result<bool, TypeError> {
        match self {
            Self::Boolean(value) => Ok(*value),
            _ => Err(TypeError::new("boolean", self.kind())),
        }
    }

    pub fn expect_number(&self) -> Result<&serde_json::Number, TypeError> {
        match self {
            Self::Number(value) => Ok(value),
            _ => Err(TypeError::new("number", self.kind())),
        }
    }

    pub fn expect_object(&self) -> Result<&Object, TypeError> {
        match self {
            Self::Object(value) => Ok(value),
            _ => Err(TypeError::new("object", self.kind())),
        }
    }

    pub fn to_json(&self) -> Result<serde_json::Value, SerializationError> {
        crate::serialize::to_json(self, false)
    }
    pub fn to_debug_json(&self) -> Result<serde_json::Value, SerializationError> {
        crate::serialize::to_json(self, true)
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

macro_rules! integer_value {
    ($($type:ty),+ $(,)?) => {$(
        impl From<$type> for Value {
            fn from(value: $type) -> Self { Self::Number(value.into()) }
        }
    )+};
}

integer_value!(i8, i16, i32, i64, u8, u16, u32, u64);
