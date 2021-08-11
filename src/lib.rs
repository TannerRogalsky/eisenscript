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
struct RuleDefinition {
    name: String,
    max_depth: Option<usize>,
    retirement_rule: Option<String>,
    weight: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct Custom {
    rule: RuleDefinition,
    actions: Vec<Action>,
}

impl Custom {
    pub fn iter<'a, R>(
        &'a self,
        ctx: Context<'a>,
        rng: &'a mut R,
    ) -> impl Iterator<Item = (Transform, Primitive)> + 'a
    where
        R: rand::Rng,
    {
        fn filter(action: &Action) -> Option<&TransformAction> {
            match action {
                Action::Set(_) => None,
                Action::Transform(tx) => Some(tx),
            }
        }

        if self.rule.max_depth == Some(ctx.depth) {
            Box::new(std::iter::empty()) as Box<dyn Iterator<Item = (Transform, Primitive)>>
        } else {
            let iter = self
                .actions
                .iter()
                .filter_map(filter)
                .flat_map(move |action| {
                    let rule = ctx.rules.get(&action.rule).unwrap();
                    action
                        .iter(ctx.tx)
                        .flat_map(|tx| rule.iter(ctx.descend(tx), rng))
                        .collect::<Vec<_>>()
                });
            Box::new(iter)
        }
    }
}

#[derive(Debug, Clone)]
struct Ambiguous {
    name: String,
    actions: Vec<Custom>,
    weights: rand_distr::WeightedIndex<f32>,
}

#[derive(Debug, Clone)]
enum Rule {
    Primitive(Primitive),
    Custom(Custom),
    Ambiguous(Ambiguous),
}

impl Rule {
    pub fn name(&self) -> &str {
        match self {
            Rule::Primitive(inner) => inner.name(),
            Rule::Custom(inner) => &inner.rule.name,
            Rule::Ambiguous(inner) => &inner.name,
        }
    }

    fn iter<R>(&self, ctx: Context, rng: &mut R) -> Vec<(Transform, Primitive)>
    where
        R: rand::Rng,
    {
        match self {
            Rule::Primitive(inner) => vec![(ctx.tx, *inner)],
            Rule::Custom(inner) => inner.iter(ctx, rng).collect(),
            Rule::Ambiguous(inner) => {
                let index = rand::Rng::sample(rng, &inner.weights);
                inner.actions[index].iter(ctx, rng).collect()
            }
        }
    }
}

struct Context<'a> {
    tx: Transform,
    depth: usize,
    rules: &'a RulesMap,
}

impl<'a> Context<'a> {
    fn new(rules: &'a RulesMap) -> Self {
        Self {
            tx: Default::default(),
            depth: 0,
            rules,
        }
    }

    fn descend(&self, tx: Transform) -> Self {
        Self {
            depth: self.depth + 1,
            tx,
            rules: self.rules,
        }
    }
}

#[derive(Debug, Clone)]
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
        .map(|p| (p.name().to_string(), Rule::Primitive(p)))
        .collect();

        Self {
            top_level: Custom {
                rule: RuleDefinition {
                    name: "Top Level".to_string(),
                    max_depth: None,
                    retirement_rule: None,
                    weight: 1.0,
                },
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
                    match rule {
                        Rule::Custom(inner) => inner,
                        _ => panic!(),
                    }
                }

                let (name, existing) = entry.remove_entry();
                let actions = vec![assert_custom(existing), assert_custom(rule)];
                let weights = actions.iter().map(|action| action.rule.weight);
                let weights = rand_distr::WeightedIndex::new(weights).unwrap();
                self.rules.insert(
                    name.clone(),
                    Rule::Ambiguous(Ambiguous {
                        name,
                        actions,
                        weights,
                    }),
                );
            }
        }
    }

    pub fn iter<'a, R: rand::Rng>(&'a self, rng: &'a mut R) -> RuleSetIterator<'a> {
        RuleSetIterator::new(self, rng)
    }
}

impl Default for RuleSet {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RuleSetIterator<'a> {
    iter: Box<dyn Iterator<Item = (Transform, Primitive)> + 'a>,
}

impl<'a> RuleSetIterator<'a> {
    pub fn new<R: rand::Rng>(rules: &'a RuleSet, rng: &'a mut R) -> Self {
        let iter = rules.top_level.iter(Context::new(&rules.rules), rng);
        Self {
            iter: Box::new(iter),
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
    tx: nalgebra::Matrix4<f32>,

    pub hue: f32,
    pub sat: f32,
    pub brightness: f32,
    pub alpha: f32,
}

impl Transform {
    pub fn translation(x: f32, y: f32, z: f32) -> Transform {
        Self {
            tx: nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(x, y, z)),
            ..Default::default()
        }
    }

    pub fn rotate_x(angle: f32) -> Transform {
        let tx = nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0., 0.5, 0.5))
            * nalgebra::Matrix4::from_axis_angle(&nalgebra::Vector3::x_axis(), angle.to_radians())
            * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0., -0.5, -0.5));
        Self {
            tx,
            ..Default::default()
        }
    }

    pub fn rotate_y(angle: f32) -> Transform {
        let tx = nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0.5, 0., 0.5))
            * nalgebra::Matrix4::from_axis_angle(&nalgebra::Vector3::y_axis(), angle.to_radians())
            * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(-0.5, 0., -0.5));
        Self {
            tx,
            ..Default::default()
        }
    }

    pub fn rotate_z(angle: f32) -> Transform {
        let tx = nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0.5, 0.5, 0.))
            * nalgebra::Matrix4::from_axis_angle(&nalgebra::Vector3::z_axis(), angle.to_radians())
            * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(-0.5, -0.5, 0.));
        Self {
            tx,
            ..Default::default()
        }
    }

    pub fn scale(x: f32, y: f32, z: f32) -> Transform {
        let tx = nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(0.5, 0.5, 0.5))
            * nalgebra::Matrix4::new_nonuniform_scaling(&nalgebra::Vector3::new(x, y, z))
            * nalgebra::Matrix4::new_translation(&nalgebra::Vector3::new(-0.5, -0.5, -0.5));
        Self {
            tx,
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
        self.tx *= rhs.tx;

        self.hue += rhs.hue;
        self.sat *= rhs.sat;
        self.brightness *= rhs.brightness;
        self.alpha *= rhs.alpha;
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
            tx: nalgebra::Matrix4::identity(),
            hue: 0.0,
            sat: 1.0,
            brightness: 1.0,
            alpha: 1.0,
        }
    }
}

impl From<Transform> for mint::ColumnMatrix4<f32> {
    fn from(t: Transform) -> Self {
        t.tx.into()
    }
}

impl From<&Transform> for mint::ColumnMatrix4<f32> {
    fn from(t: &Transform) -> Self {
        t.tx.into()
    }
}

impl approx::AbsDiffEq for Transform {
    type Epsilon = f32;

    fn default_epsilon() -> Self::Epsilon {
        f32::EPSILON
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        self.tx.abs_diff_eq(&other.tx, epsilon)
            && self.hue.abs_diff_eq(&other.hue, epsilon)
            && self.sat.abs_diff_eq(&other.sat, epsilon)
            && self.brightness.abs_diff_eq(&other.brightness, epsilon)
            && self.alpha.abs_diff_eq(&other.alpha, epsilon)
    }
}

impl approx::RelativeEq for Transform {
    fn default_max_relative() -> Self::Epsilon {
        f32::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        self.tx.relative_eq(&other.tx, epsilon, max_relative)
            && self.hue.relative_eq(&other.hue, epsilon, max_relative)
            && self.sat.relative_eq(&other.sat, epsilon, max_relative)
            && self
                .brightness
                .relative_eq(&other.brightness, epsilon, max_relative)
            && self.alpha.relative_eq(&other.alpha, epsilon, max_relative)
    }
}

impl approx::UlpsEq for Transform {
    fn default_max_ulps() -> u32 {
        f32::default_max_ulps()
    }

    fn ulps_eq(&self, other: &Self, epsilon: Self::Epsilon, max_ulps: u32) -> bool {
        self.tx.ulps_eq(&other.tx, epsilon, max_ulps)
            && self.hue.ulps_eq(&other.hue, epsilon, max_ulps)
            && self.sat.ulps_eq(&other.sat, epsilon, max_ulps)
            && self
                .brightness
                .ulps_eq(&other.brightness, epsilon, max_ulps)
            && self.alpha.ulps_eq(&other.alpha, epsilon, max_ulps)
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
    pub fn iter(&self, tx: Transform) -> TransformActionIter {
        let iter = if self.loops.is_empty() {
            let iter = std::iter::once_with(move || vec![tx]);
            Box::new(iter) as Box<dyn Iterator<Item = Vec<Transform>>>
        } else {
            let iter = self.loops.iter().map(|l| {
                (1..=l.count).scan(tx, move |state, _| {
                    *state *= l.transform;
                    Some(*state)
                })
            });
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
        let mut cmds = action.iter(Transform::default());

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
        let mut rng = rand::thread_rng();
        let mut cmds = rules.iter(&mut rng);

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
        let mut rng = rand::thread_rng();
        let mut cmds = parser.iter(&mut rng);

        assert_eq!(cmds.next(), Some((Transform::default(), Primitive::Box)));
        assert_eq!(cmds.next(), None);
    }

    #[test]
    fn hue_tx() {
        let count = 6;
        let delta = 72.;

        let source = format!("{} * {{ h {} }} box", count, delta);
        let parser = Parser::new(crate::Lexer::new(&source)).rules().unwrap();
        let mut rng = rand::thread_rng();
        let mut cmds = parser.iter(&mut rng);

        for i in 1..=count {
            assert_eq!(
                cmds.next(),
                Some((Transform::hsv(delta * i as f32, 1., 1.), Primitive::Box))
            );
        }
        assert_eq!(cmds.next(), None);
    }

    #[test]
    fn rotation_test() {
        let parser = Parser::new(crate::Lexer::new("2 * { x 1 rz 45 } box"))
            .rules()
            .unwrap();
        let mut rng = rand::thread_rng();
        let mut cmds = parser.iter(&mut rng).map(|(tx, _primitive)| tx);

        let tx = Transform::translation(1., 0., 0.) * Transform::rotate_z(45.);
        approx::assert_abs_diff_eq!(cmds.next().unwrap(), tx, epsilon = 0.001);
        let tx = tx * Transform::translation(1., 0., 0.) * Transform::rotate_z(45.);
        approx::assert_abs_diff_eq!(cmds.next().unwrap(), tx, epsilon = 0.001);

        assert_eq!(cmds.next(), None);
    }

    #[test]
    fn transform_stack() {
        let parser = Parser::new(crate::Lexer::new("{ x 1 } r1 rule r1 { box }"))
            .rules()
            .unwrap();
        let mut rng = rand::thread_rng();
        let mut cmds = parser.iter(&mut rng).map(|(tx, _primitive)| tx);

        assert_eq!(cmds.next(), Some(Transform::translation(1., 0., 0.)));
        assert_eq!(cmds.next(), None);
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

        let mut rng = rand::thread_rng();
        assert_eq!(rules.iter(&mut rng).count(), 2 * 3 * 4);
    }
}
