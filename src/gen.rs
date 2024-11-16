use std::collections::HashMap;

use codegen::Scope;

use crate::{ast, AsmLine};

pub struct CodeEmitter {
    pub patterns: Vec<ast::Pattern>,
}

impl CodeEmitter {
    pub fn gen(&self, target: ast::AstTarget) -> String {
        let asm_vec = "&mut Vec<Asm>";

        let mut scope = Scope::new();
        
        let general_func = scope.new_fn("compile")
        .arg("asm", asm_vec)
        .arg("node", "DagNode")
        .line("match node.get_opcode() {");
    
        let mut funcs_in_match = Vec::new();

        for pattern in &self.patterns {
            if !funcs_in_match.contains(&pattern.variant.mnemonic) {
                let compile_fn = format!("compile_{}", pattern.variant.mnemonic);
                let line = format!("  DagOpCode::{} => {}(asm, node),", pattern.variant.mnemonic, compile_fn);
                general_func.line(line);

                funcs_in_match.push(pattern.variant.mnemonic.to_owned());
            }
        }
        
        general_func.line("  unimplemented => todo!(\"{:?}\", node),")
            .line("}");

        let mut funcs: HashMap<String, Vec<String>> = HashMap::new();

        for pattern in &self.patterns {
            let compile_fn = format!("compile_{}", pattern.variant.mnemonic);
    
            let mut lines = Vec::new();

            // conds
    
            let mut close = 0;
    
            let construct_tabs = |close| {
                let mut out = String::new();
    
                for _ in 0..close {
                    out.push_str("  ");
                }
    
                out
            };
    
            if let Some(ls) = pattern.variant.ls {
                lines.push(format!("{}if node.is_op_{}(0) {{", construct_tabs(close), ls));
                close += 1;
            }
    
            if let Some(rs) = pattern.variant.rs {
                lines.push(format!("{}if node.is_op_{}(1) {{", construct_tabs(close), rs));
                close += 1;
            }
    
            for line in &pattern.lines {
                if let AsmLine::Asm(line) = line {
                    lines.push(format!("{}asm.push({});", construct_tabs(close), construct_assembly_build(target, line.replace("\n", ""))));
                }
                if let AsmLine::Rust(line) = line {
                    lines.push(line.to_owned());
                }
            }
    
            lines.push(format!("{}return;", construct_tabs(close)));
    
            let close_clone = close;
    
            for _ in 0..close_clone {
                close -= 1;
                lines.push(format!("{}}}", construct_tabs(close)));
            }

            if let Some(func) = funcs.get_mut(&compile_fn) {
                func.extend_from_slice(&lines);
            } else {
                funcs.insert(compile_fn, lines);
            }
        }

        for (name, lines) in &funcs {
            let func = scope.new_fn(name)
            .arg("asm", asm_vec)
            .arg("node", "DagNode");
            for line in lines {
                func.line(line);
            }
            func.line("todo!(\"not yet compilable variant: {}\", node)");
        }

        let code = scope.to_string();
        let code = code.replace("fn", "pub fn"); // make all functions public

        format!("#[allow(warnings)]\n{code}")
    }
}

fn first_to_uppercase(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn construct_assembly_build(target: ast::AstTarget, line: String) -> String {
    let mut builder = String::from("Asm::");
    
    let mut arg_string = String::new();
    
    let line = line.trim();
    let line = line.replace(", ", " ");
    let line = line.replace("[", "[ ");
    let line = line.replace("]", " ]");
    
    let mut split = line.split(' ').collect::<Vec<&str>>();
    split.reverse();
    
    let mnemonic = split.pop().expect("expected mnemonic");
    let mnemonic = first_to_uppercase(mnemonic);
    
    arg_string.push_str(&format!("Mnemonic::{}", mnemonic));
    
    let num_args = target_specific_argument_parsing(target, &mut arg_string, &mut split);
    
    arg_string.push(')');
    
    let arg_string = arg_string.replace("$out", "node.get_out().into()");
    let arg_string = arg_string.replace("$1", "node.get_op(0).into()");
    let arg_string = arg_string.replace("$2", "node.get_op(1).into()");
    
    builder.push_str(&format!("with{num_args}"));
    builder.push('(');

    builder.push_str(&arg_string);

    builder
}

fn target_specific_argument_parsing(target: ast::AstTarget, builder: &mut String, tokens: &mut Vec<&str>) -> usize{
    match target {
        ast::AstTarget::X86 => x86_specifc_arg(builder, tokens),
    }
}

fn x86_specifc_arg(builder: &mut String, tokens: &mut Vec<&str>) -> usize {
    let mut amount = 0;

    while tokens.len() > 0 {
        builder.push_str(", ");

        let Some(tok) = tokens.pop() else { break; };

        if tok != "[" { // no memory displacment
            builder.push_str(tok);
        } else { // memory displacment
            builder.push_str("MemoryDispl::new(");
            let mut first = true;
            loop {
                if !first {
                    builder.push_str(", ");
                }

                let Some(tok) = tokens.pop() else { panic!("no end to mem displacment") };

                if tok == "]" { break }
                
                match tok {
                    "+" => builder.push_str("MemoryOption::Plus"),
                    "-" => builder.push_str("MemoryOption::Minus"),
                    _ => builder.push_str(tok),
                }

                first = false;
            }
            builder.push_str(")");
        }

        amount += 1;
    }
    
    amount
}