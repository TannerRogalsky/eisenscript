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
