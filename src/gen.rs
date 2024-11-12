use codegen::Scope;

use crate::{ast, AsmLine};

pub struct CodeEmitter {
    pub patterns: Vec<ast::Pattern>,
}

impl CodeEmitter {
    pub fn gen(&self, target: ast::AstTarget) -> String {
        let mut scope = Scope::new();

        
        let general_func = scope.new_fn("compile")
        .arg("asm", "&mut Vec<McInstr>")
        .arg("node", "DagNode")
        .line("match node {");
    
        for pattern in &self.patterns {
            let compile_fn = format!("compile_{}", pattern.variant.mnemonic);
            let line = format!("  {} => {}(asm, node),", pattern.variant.mnemonic, compile_fn);
            general_func.line(line);
        }
        
        general_func.line("  unimplemented => todo!(\"{:?}\", node),")
            .line("}");

        for pattern in &self.patterns {
            let compile_fn = format!("compile_{}", pattern.variant.mnemonic);
        
            let compile_fn = scope.new_fn(&compile_fn);

            compile_fn.arg("asm", "&mut Vec<McInstr>");
            compile_fn.arg("node", "DagNode");
    
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
                compile_fn.line(format!("{}if node.is_ls_{}() {{", construct_tabs(close), ls));
                close += 1;
            }
    
            if let Some(rs) = pattern.variant.rs {
                compile_fn.line(format!("{}if node.is_rs_{}() {{", construct_tabs(close), rs));
                close += 1;
            }
    
            for line in &pattern.lines {
                if let AsmLine::Asm(line) = line {
                    compile_fn.line(format!("{}asm.push({});", construct_tabs(close), construct_assembly_build(target, line.replace("\n", ""))));
                }
                if let AsmLine::Rust(line) = line {
                    compile_fn.line(line);
                }
            }
    
            compile_fn.line(format!("{}return;", construct_tabs(close)));
    
            let close_clone = close;
    
            for _ in 0..close_clone {
                close -= 1;
                compile_fn.line(format!("{}}}", construct_tabs(close)));
            }
        }

        scope.to_string()
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
    let mut builder = String::from("Asm::new(");

    let line = line.trim();
    let line = line.replace(", ", " ");
    let line = line.replace("[", "[ ");
    let line = line.replace("]", " ]");

    let mut split = line.split(' ').collect::<Vec<&str>>();
    split.reverse();

    let mnemonic = split.pop().expect("expected mnemonic");
    let mnemonic = first_to_uppercase(mnemonic);

    builder.push_str(&format!("Mnemonic::{}", mnemonic));

    target_specific_argument_parsing(target, &mut builder, &mut split);

    builder.push(')');

    let builder = builder.replace("$out", "node.get_out().into()");
    let builder = builder.replace("$1", "node.get_op(1).into()");
    let builder = builder.replace("$2", "node.get_op(2).into()");

    builder
}

fn target_specific_argument_parsing(target: ast::AstTarget, builder: &mut String, tokens: &mut Vec<&str>) {
    match target {
        ast::AstTarget::X86 => x86_specifc_arg(builder, tokens),
    }
}

fn x86_specifc_arg(builder: &mut String, tokens: &mut Vec<&str>) {
    while tokens.len() > 0 {
        builder.push_str(", ");

        let Some(tok) = tokens.pop() else { return };

        if tok != "[" { // no memory displacment
            builder.push_str(tok);
        } else { // memory displacment
            builder.push_str("MemoryDispl::new(");
            loop {
                let Some(tok) = tokens.pop() else { panic!("no end to mem displacment") };

                if tok == "]" { break }
                
                match tok {
                    "+" => builder.push_str("MemoryOption::Plus"),
                    "-" => builder.push_str("MemoryOption::Minus"),
                    _ => builder.push_str(tok),
                }
            }
            builder.push_str(")");
        }
    }
}