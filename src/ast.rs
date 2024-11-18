use std::{fmt::Display, str::FromStr};
use pest::Parser;
use pest_derive::Parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub patterns: Vec<Pattern>,
    pub asm_parser: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub variant: Variant,
    pub lines: Vec<AsmLine>,
    pub maps: Vec<Map>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    pub var: String,
    pub ty: OpVariant, 
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
    pub ty: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpVariant {
    Gr,
    Fp,
    Imm,
    Mem
}

impl FromStr for OpVariant {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "gr" => Ok(OpVariant::Gr),
            "fp" => Ok(OpVariant::Fp),
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
            OpVariant::Fp => "fp",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AstTarget {
    X86,
}
#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct CodeParser;

pub fn process(pair: pest::iterators::Pair<Rule>) -> Pattern {
    match pair.as_rule() {
        Rule::pattern => {
            let mut pattern = Pattern {
                variant: Variant { 
                    mnemonic: String::new(), 
                    ls: None, 
                    rs: None, 
                    out: None,
                    ty: None,
                },
                maps: Vec::new(),
                lines: Vec::new(),
            };
            
            for inner_pair in pair.into_inner() {
                match inner_pair.as_rule() {
                    Rule::mnemonic => pattern.variant.mnemonic = inner_pair.as_str().to_string(),
                    Rule::inputs => {
                        if inner_pair.as_str().is_empty() { continue; }

                        let inputs = inner_pair.as_str().split(", ").map(|x| x.to_owned()).collect::<Vec<String>>();

                        if let Some(input) = inputs.get(0) {
                            pattern.variant.ls = Some(OpVariant::from_str(input).expect(&format!("invalid opvariant for ls: {}", input)))
                        }

                        if let Some(input) = inputs.get(1) {
                            pattern.variant.rs = Some(OpVariant::from_str(input).expect(&format!("invalid opvariant for rs: {}", input)))
                        }
                    },
                    Rule::optional_ty => {
                        if inner_pair.as_str().is_empty() { continue; }
                        let input = inner_pair.as_str().replace("(", "").replace(")", "").replace(" ", "");
                    
                        pattern.variant.ty = Some(input);
                    }
                    Rule::optional_output => {
                        if inner_pair.as_str().is_empty() { continue; }
                        
                        let input = inner_pair.as_str().replace("->", "").replace(" ", "");
                        let out = OpVariant::from_str(&input).expect(&format!("invalid opvariant for out: {}", input));

                        pattern.variant.out = Some(out);
                    },
                    Rule::block => process_block(&mut pattern, inner_pair),
                    Rule::map => {
                        let map = inner_pair.into_inner().as_str();
                        let map = map.replace(" ", "");

                        let map_parts = map.split(",").collect::<Vec<&str>>();
                        let tmp_name = map_parts[0];
                        let tmp_ty = OpVariant::from_str(map_parts[1]).expect("expected valid opvariant");

                        let map = Map {
                            var: tmp_name.to_string(),
                            ty: tmp_ty,
                        };

                        pattern.maps.push( map );
                    },
                    unhandled => todo!("{:?}", unhandled)
                }
            }

            pattern
        }
        unhandled => todo!("{:?}", unhandled),
    }
}

pub fn process_block(pattern: &mut Pattern, pair: pest::iterators::Pair<Rule>) {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::asm_instruction => pattern.lines.push(AsmLine::Asm(inner_pair.as_str().to_string().replace("asm ->", "").replace(";", ""))),
            Rule::rust_instruction => pattern.lines.push(AsmLine::Rust(inner_pair.as_str().to_string().replace("rust ->", ""))),
            _ => {}
        }
    }
}

pub fn parse(input: &str) -> Vec<Pattern> {
    let mut patterns = Vec::new();

    match CodeParser::parse(Rule::patterns, input) {
        Ok(pairs) => {
            for pair in pairs {
                if pair.as_rule() == Rule::patterns {
                    let inner = pair.into_inner();

                    for pair in inner { 
                        if pair.as_rule() == Rule::pattern { // else it is probably eoi
                            patterns.push( process(pair) );
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(-1);
        },
    }

    patterns
}