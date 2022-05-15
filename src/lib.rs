mod lexer;
mod parser;
mod transform;

type RulesMap = std::collections::BTreeMap<String, Rule>;
pub type Lexer<'source> = logos::Lexer<'source, lexer::Token>;
pub use parser::{Error, ErrorKind, Parser};
pub use transform::Transform;

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
    pub fn iter<'a, 'b: 'a, R>(
        &'a self,
        ctx: Context<'a>,
        ctx_mut: &'a mut ContextMut<'b, R>,
    ) -> Box<dyn Iterator<Item = (Transform, Primitive)> + 'a>
    where
        R: rand::Rng,
    {
        fn filter(action: &Action) -> Option<&TransformAction> {
            match action {
                Action::Set(_) => None,
                Action::Transform(tx) => Some(tx),
            }
        }

        let iter = self
            .actions
            .iter()
            .filter_map(filter)
            .flat_map(move |action| action.execute(&ctx, ctx_mut));
        Box::new(iter)
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

    pub fn max_depth(&self) -> Option<usize> {
        match self {
            Rule::Primitive(_) => None,
            Rule::Custom(inner) => inner.rule.max_depth,
            Rule::Ambiguous(_) => None,
        }
    }

    fn iter<'a, 'b: 'a, R>(
        &'a self,
        ctx: Context<'a>,
        ctx_mut: &'a mut ContextMut<'b, R>,
    ) -> Vec<(Transform, Primitive)>
    where
        R: rand::Rng,
    {
        match self {
            Rule::Primitive(inner) => vec![(ctx.tx, *inner)],
            Rule::Custom(inner) => inner.iter(ctx, ctx_mut).collect(),
            Rule::Ambiguous(inner) => {
                let index = rand::Rng::sample(ctx_mut.rng, &inner.weights);
                inner.actions[index].iter(ctx, ctx_mut).collect()
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

pub struct ContextMut<'a, R> {
    rng: &'a mut R,
    depths: std::collections::BTreeMap<String, usize>,
}

impl<'a, R> ContextMut<'a, R> {
    pub fn new(rng: &'a mut R) -> Self {
        Self {
            rng,
            depths: Default::default(),
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
        let rules = [
            Primitive::Box,
            Primitive::Sphere,
            Primitive::Dot,
            Primitive::Grid,
            Primitive::Cylinder,
            Primitive::Line,
            Primitive::Mesh,
            Primitive::Template,
            Primitive::Other,
        ]
        .into_iter()
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

    pub fn iter<'a, 'b: 'a, R: rand::Rng>(
        &'a self,
        ctx_mut: &'a mut ContextMut<'b, R>,
    ) -> RuleSetIterator<'a> {
        RuleSetIterator::new(self, ctx_mut)
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
    pub fn new<'b: 'a, R: rand::Rng>(
        rules: &'a RuleSet,
        ctx_mut: &'a mut ContextMut<'b, R>,
    ) -> Self {
        let iter = rules.top_level.iter(Context::new(&rules.rules), ctx_mut);
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
    fn iter(&self, tx: Transform) -> TransformActionIter {
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

    fn execute<'a, 'b: 'a, R: rand::Rng>(
        &'a self,
        ctx: &Context<'a>,
        ctx_mut: &'a mut ContextMut<'b, R>,
    ) -> Vec<(Transform, Primitive)> {
        let rule = ctx.rules.get(&self.rule).unwrap();
        if let Some(max_depth) = rule.max_depth() {
            if let Some(current) = ctx_mut.depths.get_mut(rule.name()) {
                *current = current.saturating_sub(1);
                if *current == 0 {
                    ctx_mut.depths.remove(rule.name());
                    return vec![];
                }
            } else {
                ctx_mut
                    .depths
                    .insert(rule.name().to_string(), max_depth - 1);
            }
        }
        self.iter(ctx.tx)
            .flat_map(|tx| rule.iter(ctx.descend(tx), ctx_mut))
            .collect::<Vec<_>>()
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
            .and_then(|txs| txs.into_iter().reduce(|acc, tx| acc * tx))
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
        let mut ctx = ContextMut::new(&mut rng);
        let mut cmds = rules.iter(&mut ctx);

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
        let mut ctx = ContextMut::new(&mut rng);
        let mut cmds = parser.iter(&mut ctx);

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
        let mut ctx = ContextMut::new(&mut rng);
        let mut cmds = parser.iter(&mut ctx);

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
        let mut ctx = ContextMut::new(&mut rng);
        let mut cmds = parser.iter(&mut ctx).map(|(tx, _primitive)| tx);

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
        let mut ctx = ContextMut::new(&mut rng);
        let mut cmds = parser.iter(&mut ctx).map(|(tx, _primitive)| tx);

        assert_eq!(cmds.next(), Some(Transform::translation(1., 0., 0.)));
        assert_eq!(cmds.next(), None);
    }

    #[test]
    fn recursion() {
        let parser = Parser::new(crate::Lexer::new(
            "r1
            rule r1 md 4 {
                box
                { x 1 h 20 } r1
            }",
        ))
        .rules()
        .unwrap();
        let mut rng = rand::thread_rng();
        let mut ctx = ContextMut::new(&mut rng);
        let cmds = parser.iter(&mut ctx).map(|(tx, _primitive)| tx);

        assert_eq!(cmds.count(), 4);
    }

    #[test]
    fn mixed_recursion() {
        let parser = Parser::new(crate::Lexer::new(
            r#"
            2 * { y 1 h 40 } r1
            rule r1 md 1 {
	            { x 1 h 40 } r1
	            box
            }"#,
        ))
        .rules()
        .unwrap();
        println!("{:#?}", parser);

        let mut rng = rand::thread_rng();
        let mut ctx = ContextMut::new(&mut rng);

        let rule = parser.rules.get("r1").unwrap();
        let rule = match rule {
            Rule::Custom(inner) => inner,
            _ => panic!(),
        };
        assert_eq!(rule.actions.len(), 2);

        fn filter(action: &Action) -> Option<&TransformAction> {
            match action {
                Action::Set(_) => None,
                Action::Transform(inner) => Some(inner),
            }
        }

        let action1 = rule.actions.iter().filter_map(filter).next().unwrap();
        assert_eq!(action1.rule, "r1");
        assert_eq!(
            action1.loops,
            vec![TransformationLoop {
                count: 1,
                transform: Transform::translation(1., 0., 0.) * Transform::hsv(40., 1., 1.)
            }]
        );
        let result = action1.execute(&Context::new(&parser.rules), &mut ctx);
        assert_eq!(result.len(), 1);

        let result = rule
            .actions
            .iter()
            .filter_map(filter)
            .flat_map(|action| action.execute(&Context::new(&parser.rules), &mut ctx))
            .count();
        assert_eq!(result, 2);

        assert_eq!(
            parser
                .top_level
                .iter(Context::new(&parser.rules), &mut ctx)
                .count(),
            4
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

        let mut rng = rand::thread_rng();
        let mut ctx = ContextMut::new(&mut rng);
        assert_eq!(rules.iter(&mut ctx).count(), 2 * 3 * 4);
    }
}
