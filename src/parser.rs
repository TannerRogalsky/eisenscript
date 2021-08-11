use crate::lexer::Token;

#[derive(Debug, Clone)]
pub enum ErrorKind {
    UnexpectedEOF,
    ExpectedIdentifier,
    ExpectedNumber,
    UnexpectedTransformToken,
    UnexpectedTopLevelToken,
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::UnexpectedEOF => write!(f, "Unexpected end of file."),
            ErrorKind::ExpectedIdentifier => write!(f, "Expected an identifier."),
            ErrorKind::ExpectedNumber => write!(f, "Expected a number."),
            ErrorKind::UnexpectedTransformToken => write!(f, "Unexpected transform parsing token."),
            ErrorKind::UnexpectedTopLevelToken => write!(f, "Unexpected top level token."),
        }
    }
}

#[derive(Clone)]
pub struct Error<'source> {
    pub lexer: crate::Lexer<'source>,
    pub kind: ErrorKind,
}

impl std::fmt::Debug for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::fmt::Display for Error<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let l = &self.lexer;
        write!(f, "{} {:?} => {}", self.kind, l.span(), l.slice())
    }
}

impl From<std::num::ParseIntError> for ErrorKind {
    fn from(_err: std::num::ParseIntError) -> Self {
        ErrorKind::ExpectedNumber
    }
}

impl From<std::num::ParseFloatError> for ErrorKind {
    fn from(_err: std::num::ParseFloatError) -> Self {
        ErrorKind::ExpectedNumber
    }
}

#[derive(Clone)]
pub struct Parser<'source> {
    lexer: crate::Lexer<'source>,
}

impl<'source> Parser<'source> {
    pub fn new(lexer: crate::Lexer<'source>) -> Self {
        Self { lexer }
    }

    pub fn rules(&self) -> Result<crate::RuleSet, Error<'source>> {
        let mut lexer = self.lexer.clone();
        build_rules(&mut lexer).map_err(|kind| Error { lexer, kind })
    }
}

fn next(lexer: &mut crate::Lexer) -> Result<Token, ErrorKind> {
    while let Some(token) = crate::Lexer::next(lexer) {
        if !matches!(token, Token::Error) {
            return Ok(token);
        }
    }
    Err(ErrorKind::UnexpectedEOF)
}

fn parse_action_list(token: Token, lexer: &mut crate::Lexer) -> Result<crate::Action, ErrorKind> {
    fn parse_transform(lexer: &mut crate::Lexer) -> Result<crate::Transform, ErrorKind> {
        let mut tx = crate::Transform::default();

        fn get_number(token: Token, slice: &str) -> Result<f32, ErrorKind> {
            match token {
                Token::LiteralInteger => Ok(slice.parse::<i32>()? as f32),
                Token::LiteralFloat => Ok(slice.parse()?),
                _ => Err(ErrorKind::ExpectedNumber),
            }
        }

        fn next_number(lexer: &mut crate::Lexer) -> Result<f32, ErrorKind> {
            next(lexer).and_then(|token| get_number(token, lexer.slice()))
        }

        while let Some(token) = crate::Lexer::next(lexer) {
            match token {
                Token::BracketClose => return Ok(tx),
                Token::X => tx *= crate::Transform::translation(next_number(lexer)?, 0., 0.),
                Token::Y => tx *= crate::Transform::translation(0., next_number(lexer)?, 0.),
                Token::Z => tx *= crate::Transform::translation(0., 0., next_number(lexer)?),
                Token::Rx => tx *= crate::Transform::rotate_x(next_number(lexer)?),
                Token::Ry => tx *= crate::Transform::rotate_y(next_number(lexer)?),
                Token::Rz => tx *= crate::Transform::rotate_z(next_number(lexer)?),
                Token::S => {
                    let s = next_number(lexer)?;
                    let mut temp = lexer.clone();
                    let [x, y, z] = if let Ok(sy) = next_number(&mut temp) {
                        let sz = next_number(&mut temp)?;
                        std::mem::swap(lexer, &mut temp);
                        [s, sy, sz]
                    } else {
                        [s, s, s]
                    };
                    tx *= crate::Transform::scale(x, y, z);
                }
                Token::Hue => {
                    tx.hue = next_number(lexer)?;
                }
                Token::Sat => {
                    tx.sat = next_number(lexer)?;
                }
                Token::Brightness => {
                    tx.brightness = next_number(lexer)?;
                }
                Token::Alpha => {
                    tx.alpha = next_number(lexer)?;
                }
                _ => return Err(ErrorKind::UnexpectedTransformToken),
            }
        }
        Ok(tx)
    }

    fn starts_action(token: Token) -> bool {
        matches!(token, Token::BracketOpen | Token::LiteralInteger)
    }

    let mut token = token;
    let mut loops = vec![];
    while starts_action(token) {
        let count = match token {
            Token::BracketOpen => 1,
            Token::LiteralInteger => {
                let count = std::str::FromStr::from_str(lexer.slice())?;

                assert_eq!(lexer.next(), Some(Token::Multiply));
                assert_eq!(lexer.next(), Some(Token::BracketOpen));
                count
            }
            _ => panic!(),
        };
        let transform = parse_transform(lexer)?;
        let tx_loop = crate::TransformationLoop { count, transform };
        loops.push(tx_loop);
        token = next(lexer)?;
    }
    match token {
        Token::RuleInvocation => Ok(crate::Action::Transform(crate::TransformAction {
            loops,
            rule: lexer.slice().to_string(),
        })),
        _ => Err(ErrorKind::ExpectedIdentifier),
    }
}

fn build_rules(lexer: &mut crate::Lexer) -> Result<crate::RuleSet, ErrorKind> {
    let mut is_comment = false;
    let mut rules = crate::RuleSet::new();

    while let Some(token) = crate::Lexer::next(lexer) {
        if is_comment && !matches!(token, Token::MultilineCommentEnd) {
            continue;
        }

        match token {
            Token::MultilineCommentStart => {
                is_comment = true;
            }
            Token::MultilineCommentEnd => {
                is_comment = false;
            }
            Token::RuleDefinition => {
                let name = lexer.slice().trim_start_matches("rule ").to_string();

                // TODO: parse rule modifiers
                lexer
                    .take_while(|token| !matches!(token, Token::BracketOpen))
                    .count();

                fn starts_action(token: Token) -> bool {
                    matches!(
                        token,
                        Token::BracketOpen | Token::LiteralInteger | Token::RuleInvocation
                    )
                }

                let mut actions = vec![];
                let mut next = self::next(lexer)?;
                while starts_action(next) {
                    let action = parse_action_list(next, lexer)?;
                    actions.push(action);
                    next = self::next(lexer)?;
                }
                assert_eq!(Token::BracketClose, next, "{:?}", lexer.span());
                rules.push(super::Rule {
                    max_depth: 0,
                    ty: super::RuleType::Custom(super::Custom { name, actions }),
                });
            }
            Token::Set => {
                let set_type = crate::Lexer::next(lexer).ok_or(ErrorKind::UnexpectedEOF)?;
                let set_action = match set_type {
                    Token::MaxDepth => {
                        let setting = crate::Lexer::next(lexer).ok_or(ErrorKind::UnexpectedEOF)?;
                        if let Token::LiteralInteger = setting {
                            let max_depth = lexer.slice().parse()?;
                            Ok(crate::SetAction::MaxDepth(max_depth))
                        } else {
                            Err(ErrorKind::ExpectedNumber)
                        }
                    }
                    _ => Err(ErrorKind::ExpectedIdentifier),
                }?;
                rules.add_action(crate::Action::Set(set_action));
            }
            Token::RuleInvocation => {
                let rule = lexer.slice().to_string();
                rules.add_action(crate::Action::Transform(crate::TransformAction {
                    loops: vec![],
                    rule,
                }))
            }
            Token::LiteralInteger => {
                rules.add_action(parse_action_list(token, lexer)?);
            }
            Token::BracketOpen => {
                rules.add_action(parse_action_list(token, lexer)?);
            }
            Token::Error => {}
            token => {
                eprintln!("{:?}", token);
                return Err(ErrorKind::UnexpectedTopLevelToken);
            }
        }
    }
    Ok(rules)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex() {
        let parser = Parser::new(crate::Lexer::new(INPUT));
        let rules = parser.rules();
        assert!(rules.is_ok(), "{:?}", rules);
    }

    #[test]
    fn rule_definition_single() {
        let parser = Parser::new(crate::Lexer::new("rule r1 { box }"));
        let rules = parser.rules().unwrap();
        assert_eq!(
            rules
                .rules
                .values()
                .filter(|rule| match &rule.ty {
                    crate::RuleType::Primitive(_) => false,
                    _ => true,
                })
                .count(),
            1
        );
    }

    const INPUT: &'static str = r#"/*
  Sample Torus.
*/

set maxdepth 100
r1
36  * { x -2 ry 10   } r1

rule r1 maxdepth 10 {
   2 * { y -1 } 3 * { rz 15 x 1 b -0.9 h -20  } r2
   { y 1 h 12 a 0.9  rx 36 }  r1
}

rule r2 {
   { s 0.9 0.1 1.1 hue 10 } box // a comment
}

rule r2 w 2 {
   { hue 113 sat 19 a 23 s 0.1 0.9 1.1 } box
}
"#;
}
