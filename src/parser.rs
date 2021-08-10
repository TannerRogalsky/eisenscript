use crate::lexer::Token;

#[derive(Debug)]
pub enum Error {
    UnexpectedEOF,
    ExpectedIdentifier,
    ExpectedNumber,
    UnexpectedToken(String),
}

impl From<std::num::ParseIntError> for Error {
    fn from(_err: std::num::ParseIntError) -> Self {
        Error::ExpectedNumber
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(_err: std::num::ParseFloatError) -> Self {
        Error::ExpectedNumber
    }
}

pub struct Parser<'source> {
    lexer: crate::Lexer<'source>,
}

impl<'source> Parser<'source> {
    pub fn new(lexer: crate::Lexer<'source>) -> Self {
        Self { lexer }
    }

    pub fn rules(&self) -> Result<crate::RuleSet, Error> {
        let mut lexer = self.lexer.clone();
        let mut is_comment = false;
        let mut rules = crate::RuleSet::new();

        while let Some(token) = lexer.next() {
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
                    (&mut lexer)
                        .take_while(|token| match token {
                            Token::BracketOpen => false,
                            _ => true,
                        })
                        .count();

                    fn starts_action(token: Token) -> bool {
                        match token {
                            Token::BracketOpen | Token::LiteralInteger | Token::RuleInvocation => {
                                true
                            }
                            _ => false,
                        }
                    }

                    let mut actions = vec![];
                    let mut next = lexer.next().unwrap();
                    while starts_action(next) {
                        let action = Self::parse_action_list(next, &mut lexer)?;
                        actions.push(action);
                        next = lexer.next().ok_or(Error::UnexpectedEOF)?;
                    }
                    assert_eq!(Token::BracketClose, next, "{:?}", lexer.span());
                    rules.push(super::Rule {
                        max_depth: 0,
                        ty: super::RuleType::Custom(super::Custom { name, actions }),
                    });
                }
                Token::Set => {
                    let set_type = lexer.next().ok_or(Error::UnexpectedEOF)?;
                    let set_action = match set_type {
                        Token::MaxDepth => {
                            let setting = lexer.next().ok_or(Error::UnexpectedEOF)?;
                            if let Token::LiteralInteger = setting {
                                let max_depth = lexer.slice().parse()?;
                                Ok(crate::SetAction::MaxDepth(max_depth))
                            } else {
                                Err(Error::ExpectedNumber)
                            }
                        }
                        _ => Err(Error::ExpectedIdentifier),
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
                    rules.add_action(Self::parse_action_list(token, &mut lexer)?);
                }
                Token::BracketOpen => {
                    rules.add_action(Self::parse_action_list(token, &mut lexer)?);
                }
                _ => {
                    eprintln!("{:?}", lexer.span());
                    return Err(Error::UnexpectedToken(lexer.slice().to_string()));
                }
            }
        }

        Ok(rules)
    }

    fn parse_action_list(token: Token, lexer: &mut crate::Lexer) -> Result<crate::Action, Error> {
        fn parse_transform(lexer: &mut crate::Lexer) -> Result<crate::Transform, Error> {
            let mut tx = crate::Transform::default();

            fn get_number((token, slice): (Token, &str)) -> Result<f32, Error> {
                match token {
                    Token::LiteralInteger => Ok(slice.parse::<i32>()? as f32),
                    Token::LiteralFloat => Ok(slice.parse()?),
                    _ => Err(Error::ExpectedNumber),
                }
            }

            fn next<'a>(lexer: &mut crate::Lexer<'a>) -> Result<(Token, &'a str), Error> {
                crate::Lexer::next(lexer)
                    .ok_or(Error::UnexpectedEOF)
                    .map(|token| (token, lexer.slice()))
            }

            fn next_number(lexer: &mut crate::Lexer) -> Result<f32, Error> {
                next(lexer).and_then(get_number)
            }

            while let Some(token) = crate::Lexer::next(lexer) {
                match token {
                    Token::BracketClose => return Ok(tx),
                    Token::X => tx.x = next_number(lexer)?,
                    Token::Y => tx.y = next_number(lexer)?,
                    Token::Z => tx.z = next_number(lexer)?,
                    Token::Rx => tx.rx = next_number(lexer)?,
                    Token::Ry => tx.ry = next_number(lexer)?,
                    Token::Rz => tx.rz = next_number(lexer)?,
                    Token::S => {
                        let s = next_number(lexer)?;
                        let mut temp = lexer.clone();
                        if let Ok(sy) = next_number(&mut temp) {
                            let sz = next_number(&mut temp)?;
                            tx.sx = s;
                            tx.sy = sy;
                            tx.sz = sz;
                            std::mem::swap(lexer, &mut temp);
                        } else {
                            tx.sx = s;
                            tx.sy = s;
                            tx.sz = s;
                        }
                    }
                    Token::Brightness | Token::Hue | Token::Sat | Token::Alpha => {
                        let num = next(lexer).and_then(get_number)?;
                        assert!(!num.is_nan());
                    }
                    _ => {
                        eprintln!("{:?} => {:?}", lexer.slice(), lexer.span());
                        unimplemented!()
                    }
                }
            }
            Ok(tx)
        }

        fn starts_action(token: Token) -> bool {
            match token {
                Token::BracketOpen | Token::LiteralInteger => true,
                _ => false,
            }
        }

        let mut token = token;
        let mut loops = vec![];
        while starts_action(token) {
            let count = match token {
                Token::BracketOpen => 1,
                Token::LiteralInteger => {
                    let count = std::str::FromStr::from_str(lexer.slice()).unwrap();
                    assert_eq!(lexer.next(), Some(Token::Multiply));
                    assert_eq!(lexer.next(), Some(Token::BracketOpen));
                    count
                }
                _ => panic!(),
            };
            let transform = parse_transform(lexer).unwrap();
            let tx_loop = crate::TransformationLoop { count, transform };
            loops.push(tx_loop);
            token = crate::Lexer::next(lexer).unwrap();
        }
        match token {
            Token::RuleInvocation => Ok(crate::Action::Transform(crate::TransformAction {
                loops,
                rule: lexer.slice().to_string(),
            })),
            _ => Err(Error::ExpectedIdentifier),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex() {
        let parser = Parser::new(crate::Lexer::new(INPUT));
        let rules = parser.rules();
        assert!(rules.is_ok());
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
