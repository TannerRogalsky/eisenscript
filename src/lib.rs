mod lexer;
mod parser;

pub type Lexer<'source> = logos::Lexer<'source, lexer::Token>;
pub use parser::Parser;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum Primitive {
    Box,
    Sphere,
    Dot,
    Grid,
    Cylinder,
    Line,
    Mesh,
    Template,
    Other,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Custom {
    name: String,
    actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum RuleType {
    Primitive(Primitive),
    Custom(Custom),
    // Ambiguous,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Rule {
    pub max_depth: usize,
    pub ty: RuleType,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Ruleset {
    top_level: Custom,
    rules: Vec<Rule>,
}

impl Ruleset {
    pub fn new() -> Self {
        Self {
            top_level: Custom {
                name: "Top Level".to_string(),
                actions: vec![],
            },
            rules: vec![],
        }
    }

    pub fn push(&mut self, rule: Rule) {
        self.rules.push(rule);
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Transform {
    x: f32,
    y: f32,
    z: f32,
    rx: f32,
    ry: f32,
    rz: f32,
    sx: f32,
    sy: f32,
    sz: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            rx: 0.0,
            ry: 0.0,
            rz: 0.0,
            sx: 1.0,
            sy: 1.0,
            sz: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct TransformationLoop {
    count: usize,
    transform: Transform,
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum SetAction {
    MaxDepth(usize),
    MaxObjects(usize),
    MinSize(f32),
    MaxSize(f32),
    Seed(usize),
    ResetSeed,
    Background(String),
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub enum Action {
    Set(SetAction),
    Transform {
        loops: Vec<TransformationLoop>,
        rule: String,
    },
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
