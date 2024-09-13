use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r" ")] // Ignore this regex pattern between tokens
enum Token_ {
    // Tokens can be literal strings, of any length.
    #[regex("fn |pub fn ")]
    DeclareFn,
    #[regex("\".*\"")]
    String,
    #[regex("struct |pub struct ")]
    DeclareStruct,
    #[regex("let |let mut ")]
    DeclareVar,
    #[token("::")]
    PrantInvoked,
    #[token(".")]
    InstanceInvoked,
    #[token("enum")]
    DeclareEnum,
    #[token("(")]
    LBrack,
    #[token(")")]
    RBrack,
    #[token("{")]
    LCBrack,
    #[regex(": ?[a-zA-Z]+")]
    TypeHint,
    #[regex("<[a-zA-Z]>")]
    Type,
    #[token("}")]
    RCBrack,
    #[token("<=")]
    GreatEq,
    #[token(">=")]
    LesssEq,
    #[token("<")]
    Lesser,
    #[token(">")]
    Greater,
    #[regex("[0-9]+")]
    Number,
    #[regex("[a-zA-Z_][a-zA-Z_0-9]+")]
    Name,
}
