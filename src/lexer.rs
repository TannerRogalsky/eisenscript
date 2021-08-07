use logos::Logos;

#[derive(Logos, Debug, PartialEq, Copy, Clone)]
pub enum Token {
    #[token("/*")]
    MultilineCommentStart,
    #[token("*/")]
    MultilineCommentEnd,
    #[regex("//.*", logos::skip)]
    Comment,

    #[regex("rule [a-zA-Z]+[a-zA-Z0-9]*")]
    RuleDefinition,
    #[regex("[a-zA-Z]+[a-zA-Z0-9]*")]
    RuleInvocation,

    #[token("set")]
    Set,

    #[token("{")]
    BracketOpen,
    #[token("}")]
    BracketClose,

    #[token(">")]
    MoreThan,
    #[token("*")]
    Multiply,

    #[regex("[+-]?[0-9]+", priority = 2)]
    LiteralInteger,
    #[regex("[+-]?[0-9]*[.]?[0-9]+")]
    LiteralFloat,

    #[token("maxdepth")]
    #[token("md")]
    MaxDepth,

    #[token("weight")]
    #[token("w")]
    Weight,

    #[token("hue")]
    #[token("h")]
    Hue,

    #[token("brightness")]
    #[token("b")]
    Brightness,

    #[token("alpha")]
    #[token("a")]
    Alpha,

    #[token("color")]
    #[token("c")]
    Color,

    #[token("reflect")]
    Reflect,
    #[token("blend")]
    Blend,
    #[token("matrix")]
    Matrix,
    #[token("sat")]
    Sat,
    #[token("v")]
    V,
    #[token("x")]
    X,
    #[token("y")]
    Y,
    #[token("z")]
    Z,
    #[token("rx")]
    Rx,
    #[token("ry")]
    Ry,
    #[token("rz")]
    Rz,
    #[token("s")]
    S,
    #[token("fx")]
    Fx,
    #[token("fy")]
    Fy,
    #[token("fz")]
    Fz,

    #[error]
    #[regex(r"[ \t\n\f]+", logos::skip)]
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut _lex: logos::Lexer<Token> = Token::lexer(INPUT);
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
