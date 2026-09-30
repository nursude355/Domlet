#[derive(Debug, PartialEq)]
pub struct Component {
    pub offset: usize,
    pub name: String,
    pub root_tag: &'static str,
    pub title: Option<String>,
    pub properties: Vec<Property>,
    pub callbacks: Vec<Callback>,
    pub children: Vec<Element>,
}

#[derive(Debug, PartialEq)]
pub struct Property {
    pub offset: usize,
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
    pub offset: usize,
    pub kind: String,
    pub id: Option<String>,
    pub properties: Vec<ElementProperty>,
    pub handlers: Vec<Handler>,
    pub children: Vec<Element>,
}

#[derive(Debug, PartialEq)]
pub struct Handler {
    pub offset: usize,
    pub event: String,
    pub callback: String,
}

#[derive(Debug, PartialEq)]
pub struct Callback {
    pub offset: usize,
    pub name: String,
}

#[derive(Debug, PartialEq)]
pub struct ElementProperty {
    pub offset: usize,
    pub name: String,
    pub value: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    String(String),
    Bool(bool),
    Number(String),
    Identifier(String),
    NotIdentifier(String),
}
