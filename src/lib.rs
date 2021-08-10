mod lexer;
mod parser;

type RulesMap = std::collections::BTreeMap<String, Rule>;
pub type Lexer<'source> = logos::Lexer<'source, lexer::Token>;
use itertools::Itertools;
pub use parser::{Error, ErrorKind, Parser};

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
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

impl Primitive {
    pub fn name(&self) -> &str {
        match self {
            Primitive::Box => "box",
            Primitive::Sphere => "sphere",
            Primitive::Dot => "dot",
            Primitive::Grid => "grid",
            Primitive::Cylinder => "cyliner",
            Primitive::Line => "line",
            Primitive::Mesh => "mesh",
            Primitive::Template => "template",
            Primitive::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Custom {
    name: String,
    actions: Vec<Action>,
}

impl Custom {
    pub fn iter<'a>(
        &'a self,
        rules: &'a RulesMap,
    ) -> impl Iterator<Item = (Transform, Primitive)> + 'a {
        fn filter(action: &Action) -> Option<&TransformAction> {
            match action {
                Action::Set(_) => None,
                Action::Transform(tx) => Some(tx),
            }
        }

        self.actions
            .iter()
            .filter_map(filter)
            .flat_map(move |action| {
                let rule = rules.get(&action.rule).unwrap();
                let result = action.iter().flat_map(move |tx| match &rule.ty {
                    RuleType::Primitive(inner) => Box::new(std::iter::once((tx, *inner)))
                        as Box<dyn Iterator<Item = (Transform, Primitive)>>,
                    RuleType::Custom(inner) => Box::new(inner.iter(rules)),
                    RuleType::Ambiguous(_) => unimplemented!(),
                });
                result
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Ambiguous {
    name: String,
    actions: Vec<(usize, Custom)>,
}

#[derive(Debug, Clone, PartialEq)]
enum RuleType {
    Primitive(Primitive),
    Custom(Custom),
    Ambiguous(Ambiguous),
}

#[derive(Debug, Clone, PartialEq)]
struct Rule {
    pub max_depth: usize,
    pub ty: RuleType,
}

impl Rule {
    pub fn name(&self) -> &str {
        match &self.ty {
            RuleType::Primitive(inner) => inner.name(),
            RuleType::Custom(inner) => &inner.name,
            RuleType::Ambiguous(inner) => &inner.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleSet {
    top_level: Custom,
    rules: RulesMap,
}

impl RuleSet {
    pub fn new() -> Self {
        let rules = std::array::IntoIter::new([
            Primitive::Box,
            Primitive::Sphere,
            Primitive::Dot,
            Primitive::Grid,
            Primitive::Cylinder,
            Primitive::Line,
            Primitive::Mesh,
            Primitive::Template,
            Primitive::Other,
        ])
        .map(|p| {
            (
                p.name().to_string(),
                Rule {
                    max_depth: 0,
                    ty: RuleType::Primitive(p),
                },
            )
        })
        .collect();

        Self {
            top_level: Custom {
                name: "Top Level".to_string(),
                actions: vec![],
            },
            rules,
        }
    }

    fn add_action(&mut self, action: Action) {
        self.top_level.actions.push(action);
    }

    fn push(&mut self, rule: Rule) {
        use std::collections::btree_map::Entry;
        match self.rules.entry(rule.name().to_string()) {
            Entry::Vacant(entry) => {
                entry.insert(rule);
            }
            Entry::Occupied(entry) => {
                fn assert_custom(rule: Rule) -> Custom {
                    match rule.ty {
                        RuleType::Custom(inner) => inner,
                        _ => panic!(),
                    }
                }

                let (name, existing) = entry.remove_entry();
                self.rules.insert(
                    name,
                    Rule {
                        max_depth: 0,
                        ty: RuleType::Ambiguous(Ambiguous {
                            name: existing.name().to_string(),
                            actions: vec![(0, assert_custom(existing)), (0, assert_custom(rule))],
                        }),
                    },
                );
            }
        }
    }

    pub fn iter(&self) -> RuleSetIterator {
        RuleSetIterator::new(self)
    }
}

pub struct RuleSetIterator<'a> {
    iter: Box<dyn Iterator<Item = (Transform, Primitive)> + 'a>,
}

impl<'a> RuleSetIterator<'a> {
    pub fn new(rules: &'a RuleSet) -> Self {
        Self {
            iter: Box::new(rules.top_level.iter(&rules.rules)),
        }
    }
}

impl Iterator for RuleSetIterator<'_> {
    type Item = (Transform, Primitive);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub rx: f32,
    pub ry: f32,
    pub rz: f32,
    pub sx: f32,
    pub sy: f32,
    pub sz: f32,
    pub hue: f32,
    pub sat: f32,
    pub brightness: f32,
    pub alpha: f32,
}

impl Transform {
    pub fn translation(x: f32, y: f32, z: f32) -> Transform {
        Self {
            x,
            y,
            z,
            ..Default::default()
        }
    }

    pub fn hsv(hue: f32, sat: f32, brightness: f32) -> Transform {
        Self {
            hue,
            sat,
            brightness,
            ..Default::default()
        }
    }
}

impl std::ops::MulAssign for Transform {
    fn mul_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;

        self.hue += rhs.hue;
        self.sat *= rhs.sat;
        self.brightness *= rhs.brightness;
        self.alpha *= rhs.alpha;
    }
}

impl std::ops::MulAssign<f32> for Transform {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;

        self.hue *= rhs;
        // self.sat *= rhs;
        // self.brightness *= rhs;
        // self.alpha *= rhs;
    }
}

impl std::ops::Mul<f32> for Transform {
    type Output = Self;

    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}

impl std::ops::Mul for Transform {
    type Output = Self;

    fn mul(mut self, rhs: Self) -> Self::Output {
        self *= rhs;
        self
    }
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
            hue: 0.0,
            sat: 1.0,
            brightness: 1.0,
            alpha: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct TransformationLoop {
    count: usize,
    transform: Transform,
}

#[derive(Debug, Clone, PartialEq)]
struct TransformAction {
    loops: Vec<TransformationLoop>,
    rule: String,
}

impl TransformAction {
    pub fn iter(&self) -> TransformActionIter {
        let iter = if self.loops.is_empty() {
            let iter = std::iter::once_with(|| vec![Transform::default()]);
            Box::new(iter) as Box<dyn Iterator<Item = Vec<Transform>>>
        } else {
            let iter = self
                .loops
                .iter()
                .map(|l| (1..=l.count).map(move |i| l.transform * i as f32));
            Box::new(itertools::Itertools::multi_cartesian_product(iter))
        };
        TransformActionIter { iter }
    }
}

struct TransformActionIter<'a> {
    iter: Box<dyn Iterator<Item = Vec<Transform>> + 'a>,
}

impl Iterator for TransformActionIter<'_> {
    type Item = Transform;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .and_then(|txs| txs.into_iter().fold1(|acc, tx| acc * tx))
    }
}

#[allow(unused)]
#[derive(Debug, Clone, PartialOrd, PartialEq)]
enum SetAction {
    MaxDepth(usize),
    MaxObjects(usize),
    MinSize(f32),
    MaxSize(f32),
    Seed(usize),
    ResetSeed,
    Background(String),
}

#[derive(Debug, Clone, PartialEq)]
enum Action {
    Set(SetAction),
    Transform(TransformAction),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_fn() {
        let action = TransformAction {
            loops: vec![
                TransformationLoop {
                    count: 2,
                    transform: Transform::translation(2., 0., 0.),
                },
                TransformationLoop {
                    count: 2,
                    transform: Transform::translation(0., 2., 0.),
                },
            ],
            rule: "".to_string(),
        };
        let mut cmds = action.iter();

        assert_eq!(cmds.next(), Some(Transform::translation(2., 2., 0.)));
        assert_eq!(cmds.next(), Some(Transform::translation(2., 4., 0.)));
        assert_eq!(cmds.next(), Some(Transform::translation(4., 2., 0.)));
        assert_eq!(cmds.next(), Some(Transform::translation(4., 4., 0.)));
        assert_eq!(cmds.next(), None);
    }

    #[test]
    fn basic_tx() {
        let rules = Parser::new(crate::Lexer::new("{ x 2 } box"))
            .rules()
            .unwrap();
        let mut cmds = rules.iter();

        assert_eq!(
            cmds.next(),
            Some((Transform::translation(2., 0., 0.), Primitive::Box))
        );
        assert_eq!(cmds.next(), None);
    }

    #[test]
    fn basic_custom() {
        let parser = Parser::new(crate::Lexer::new("r1 rule r1 { box }"))
            .rules()
            .unwrap();
        let mut cmds = parser.iter();

        assert_eq!(cmds.next(), Some((Transform::default(), Primitive::Box)));
        assert_eq!(cmds.next(), None);
    }

    #[test]
    fn color_tx() {
        let source = "6 * { h 72 } box";
        let parser = Parser::new(crate::Lexer::new(source)).rules().unwrap();
        let mut cmds = parser.iter();

        assert_eq!(
            cmds.next(),
            Some((Transform::hsv(72., 1., 1.), Primitive::Box))
        );
    }

    #[test]
    fn custom_rule_lookup() {
        const INPUT: &'static str = r#"
3 * { x 2 h 40 } 2 * { y 2 h 40 } 4 * { z 2 h 40 } r1

rule r1 {
	box
}
"#;

        let parser = Parser::new(crate::Lexer::new(INPUT));
        let rules = parser.rules().unwrap();

        assert_eq!(rules.iter().count(), 2 * 3 * 4);
    }
}
