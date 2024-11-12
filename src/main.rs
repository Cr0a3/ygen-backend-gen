use pest::Parser;
use pest_derive::Parser;
use std::str::FromStr;

pub mod ast;
use crate::ast::*;

pub mod gen;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct CodeParser;

fn process(pair: pest::iterators::Pair<Rule>) -> ast::Pattern {
    match pair.as_rule() {
        Rule::pattern => {
            let mut pattern = Pattern {
                variant: Variant { 
                    mnemonic: String::new(), 
                    ls: None, 
                    rs: None, 
                    out: None 
                },
                lines: Vec::new(),
            };
            
            for inner_pair in pair.into_inner() {
                match inner_pair.as_rule() {
                    Rule::mnemonic => pattern.variant.mnemonic = inner_pair.as_str().to_string(),
                    Rule::inputs => {
                        let inputs = inner_pair.as_str().split(", ").map(|x| x.to_owned()).collect::<Vec<String>>();

                        if let Some(input) = inputs.get(0) {
                            pattern.variant.ls = Some(OpVariant::from_str(input).expect(&format!("invalid opvariant: {}", input)))
                        }

                        if let Some(input) = inputs.get(1) {
                            pattern.variant.rs = Some(OpVariant::from_str(input).expect(&format!("invalid opvariant: {}", input)))
                        }
                    },
                    Rule::optional_output => {
                        let input = inner_pair.as_str().replace("->", "").replace(" ", "");
                        let out = OpVariant::from_str(&input).expect(&format!("invalid opvariant: {}", input));

                        pattern.variant.out = Some(out);
                    },
                    Rule::block => process_block(&mut pattern, inner_pair),
                    unhandled => todo!("{:?}", unhandled)
                }
            }

            pattern
        }
        unhandled => todo!("{:?}", unhandled),
    }
}

fn process_block(pattern: &mut ast::Pattern, pair: pest::iterators::Pair<Rule>) {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            Rule::asm_instruction => pattern.lines.push(AsmLine::Asm(inner_pair.as_str().to_string().replace("asm ->", ""))),
            Rule::rust_instruction => pattern.lines.push(AsmLine::Rust(inner_pair.as_str().to_string())),
            _ => {}
        }
    }
}

fn main() {
    let input = r#"def Pat<Add gr, gr -> gr> {
    asm -> lea $out, [$1 + $2]
}

def Pat<Add gr, imm -> gr> {
    rust -> if $out == $gr {
        asm -> add $out, $2
    rust -> } else {
        asm -> mov $out, $2
        asm -> add $out, $1
    rust -> }
}

def Pat<Add imm, imm -> gr> {
    asm -> mov $out, $2
    asm -> add $out, $1
}

def Pat<Add mem, gr -> gr> {
    asm -> mov $out, $1
    asm -> add $out, $2
}

def Pat<Add mem, mem -> gr> {
    asm -> mov $out, $1
    asm -> add $out, $2
}

def Pat<Add mem, mem -> mem> {
    asm -> mov $t1, $1
    asm -> add $t1, $2
    asm -> mov $out, $t1
}"#;

    let mut patterns = Vec::new();

    match CodeParser::parse(Rule::pattern, input) {
        Ok(pairs) => {
            for pair in pairs {
                patterns.push( process(pair) );
            }
        }
        Err(e) => eprintln!("{}", e),
    }

    let emiter = gen::CodeEmitter {
        patterns: patterns
    };

    println!("{}", emiter.gen(AstTarget::X86));
}
