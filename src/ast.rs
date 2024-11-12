use std::{fmt::Display, str::FromStr};


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub patterns: Vec<Pattern>,
    pub asm_parser: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub variant: Variant,
    pub lines: Vec<AsmLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsmLine {
    Rust(String),
    Asm(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub mnemonic: String,
    pub ls: Option<OpVariant>,
    pub rs: Option<OpVariant>,
    pub out: Option<OpVariant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpVariant {
    Gr,
    Imm,
    Mem
}

impl FromStr for OpVariant {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "gr" => Ok(OpVariant::Gr),
            "imm" => Ok(OpVariant::Imm),
            "mem" => Ok(OpVariant::Mem),
            _ => Err(()),
        }
    }
}

impl Display for OpVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            OpVariant::Gr => "gr",
            OpVariant::Imm => "imm",
            OpVariant::Mem => "mem",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstTarget {
    X86,
}