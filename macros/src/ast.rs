#[derive(Debug, PartialEq)]
pub struct Component {
    pub name: String,
    pub root_tag: &'static str,
    pub title: Option<String>,
    pub properties: Vec<Property>,
    pub callbacks: Vec<String>,
    pub children: Vec<Element>,
}

#[derive(Debug, PartialEq)]
pub struct Property {
    pub name: String,
    pub kind: PropertyKind,
    pub initial: Value,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropertyKind {
    String,
    Bool,
    Int,
    Float,
}

#[derive(Debug, PartialEq)]
pub struct Element {
    pub kind: String,
    pub id: Option<String>,
    pub properties: Vec<(String, Value)>,
    pub handlers: Vec<Handler>,
    pub children: Vec<Element>,
}

#[derive(Debug, PartialEq)]
pub struct Handler {
    pub event: String,
    pub callback: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    String(String),
    Bool(bool),
    Number(String),
    Identifier(String),
}
