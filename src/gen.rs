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
        .arg("module", "&mut crate::IR::Module")
        .line("match node.get_opcode() {");
    
        let mut funcs_in_match = Vec::new();

        for pattern in &self.patterns {
            if !funcs_in_match.contains(&pattern.variant.mnemonic) {
                let compile_fn = format!("compile_{}", pattern.variant.mnemonic.replace("(_)", ""));
                let line = format!("  DagOpCode::{} => {}(asm, node, module),", pattern.variant.mnemonic, compile_fn);
                general_func.line(line);

                funcs_in_match.push(pattern.variant.mnemonic.to_owned());
            }
        }
        
        general_func.line("  unimplemented => todo!(\"{:?}\", node),")
            .line("}");

        let mut funcs: HashMap<String, Vec<String>> = HashMap::new();

        for pattern in &self.patterns {
            let compile_fn = format!("compile_{}", pattern.variant.mnemonic.replace("(_)", ""));
    
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
                if ls != ast::OpVariant::Any {
                    lines.push(format!("{}if node.is_op_{}(0) {{", construct_tabs(close), ls));
                    close += 1;
                }
            }
    
            if let Some(rs) = pattern.variant.rs {
                if rs != ast::OpVariant::Any {
                    lines.push(format!("{}if node.is_op_{}(1) {{", construct_tabs(close), rs));
                    close += 1;
                }
            }

            if let Some(op3) = pattern.variant.op3 {
                if op3 != ast::OpVariant::Any {
                    lines.push(format!("{}if node.is_op_{}(2) {{", construct_tabs(close), op3));
                    close += 1;
                }
            }

            if let Some(out) = pattern.variant.out {
                if out != ast::OpVariant::Any {
                    lines.push(format!("{} if node.is_out_{out}() {{", construct_tabs(close)));
                    close += 1;
                }
            }

            if let Some(ty) = &pattern.variant.ty {
                if ty.contains("<") && ty.contains(">") {
                    // vector type

                    let ty = ty.replace(">", "").replace("<", "");

                    let ty = ty.split("x").collect::<Vec<&str>>();

                    let size = ty.get(0).expect("expected size for vector types");
                    let ty = ty.get(1).expect("expected type");

                    lines.push(format!("{}if node.is_ty(crate::IR::TypeMetadata::Vector(crate::IR::VecTy {{ size: {size}, ty: crate::IR::StdTypeMetadata::{ty}}})) {{", construct_tabs(close)));
                } else {
                    match ty.as_str() {
                        "int" => lines.push(format!("{}if node.get_ty().intenger() {{", construct_tabs(close))),
                        "signed" => lines.push(format!("{}if node.get_ty().signed() {{", construct_tabs(close))),
                        "unsigned" => lines.push(format!("{}if !node.get_ty().signed() {{", construct_tabs(close))),
                        "float" => lines.push(format!("{}if node.get_ty().float() {{", construct_tabs(close))),
                        "no_float" => lines.push(format!("{}if !node.get_ty().float() {{", construct_tabs(close))),
                        _ => lines.push(format!("{}if node.is_ty(crate::IR::TypeMetadata::{}) {{", construct_tabs(close), ty)),
                    }
                }
                close += 1; 
            }

            construct_asm(target, &pattern, &mut lines, close, construct_tabs);
    
            if let Some(hook) = &pattern.hook {
                lines.push(format!("{}{hook}(asm, node);", construct_tabs(close)))
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
            .arg("node", "DagNode")
            .arg("module", "&mut crate::IR::Module");
            for line in lines {
                func.line(line);
            }
            func.line("todo!(\"not yet compilable variant: {} ({})\", node, node.get_ty())");
        }

        // now handle the temporarys
        self.gen_tmps(&mut scope, target);

        // handle the overwrites
        self.gen_overwrittes(&mut scope, target);

        let code = scope.to_string();
        let code = code.replace("fn", "pub fn"); // make all functions public

        format!("#[allow(warnings)]\n{code}")
    }

    fn construct_cond(&self, pat: &ast::Pattern) -> String {
        let mut cond = String::new();

        cond.push_str(&format!("if let DagOpCode::{} = node.get_opcode()  {{", pat.variant.mnemonic));

        cond.push_str("if true ");

        if let Some(ls) = pat.variant.ls {
            if ls != ast::OpVariant::Any {
                cond.push_str(&format!(" && node.is_op_{ls}(0)"));
            }
        }

        if let Some(rs) = pat.variant.rs {
            if rs != ast::OpVariant::Any {
                cond.push_str(&format!(" && node.is_op_{rs}(1)"));
            }
        }
        
        if let Some(op3) = pat.variant.op3 {
            if op3 != ast::OpVariant::Any {
                cond.push_str(&format!(" && node.is_op_{op3}(2)"));
            }
        }

        if let Some(out) = pat.variant.out {
            if out != ast::OpVariant::Any {
                cond.push_str(&format!(" && node.is_out_{out}()"));
            }
        }
        
        if let Some(ty) = &pat.variant.ty {
            if ty.contains("<") && ty.contains(">") {
                // vector type

                let ty = ty.replace(">", "").replace("<", "");

                let ty = ty.split("x").collect::<Vec<&str>>();

                let size = ty.get(0).expect("expected size for vector types");
                let ty = ty.get(1).expect("expected type");

                cond.push_str(&format!("&& node.is_ty(crate::IR::TypeMetadata::Vector(crate::IR::VecTy {{ size: {size}, ty: crate::IR::StdTypeMetadata::{ty}}}))"));
            } else {
                match ty.as_str() {
                    "int" => cond.push_str(" && node.get_ty().intenger()"),
                    "signed" => cond.push_str(" && node.get_ty().signed()"),
                    "unsigned" => cond.push_str(" && !node.get_ty().signed()"),
                    "float" => cond.push_str(" && node.get_ty().float()"),
                    "no_float" => cond.push_str(" && !node.get_ty().float()"),
                    _ => cond.push_str(&format!(" && node.is_ty(crate::IR::TypeMetadata::{})",  ty)),
                }
            }
        }

        cond
    }

    fn gen_tmps(&self, scope: &mut Scope, _target: ast::AstTarget) {
        let tmp_req_func = scope.new_fn("tmps")
            .arg("node", "&dag::DagNode")
            .ret("Vec<dag::DagTmpInfo>");

        for pat in &self.patterns {
            tmp_req_func.line(format!("{} {{", self.construct_cond(&pat)));

            tmp_req_func.line("\tlet mut tmps = Vec::new();");

            if let Some(_) = pat.variant.ls {
                tmp_req_func.line("\tlet ls_tmps = OperationHandler::new().tmp(&node.get_op(0), 0xF0);");
                tmp_req_func.line("\ttmps.extend_from_slice(&ls_tmps);");
            }
            if let Some(_) = pat.variant.rs {
                tmp_req_func.line("\tlet ls_tmps = OperationHandler::new().tmp(&node.get_op(1), 0xF1);");
                tmp_req_func.line("\ttmps.extend_from_slice(&ls_tmps);");
            }

            for tmp in &pat.maps {
                let num = tmp.var.replace("%t", "");

                tmp_req_func.line(format!("\tlet mut tmp = dag::DagTmpInfo::new({num}, node.get_ty());"));

                let func = match tmp.ty {
                    ast::OpVariant::Gr => "tmp.require_gr()",
                    ast::OpVariant::Fp => "tmp.require_fp()",
                    ast::OpVariant::Mem => "tmp.require_mem()",
                    ast::OpVariant::Imm => panic!("tmps cannot have imm as their type"),
                    ast::OpVariant::Any => panic!("tmporarys cannot have any type"),
                };

                tmp_req_func.line("\ttmp.size = node.get_ty();");

                tmp_req_func.line(format!("\t{func};"));
                tmp_req_func.line("\ttmps.push(tmp);");

            }
            tmp_req_func.line("\treturn tmps;");
            tmp_req_func.line("}\t}");
        }
        tmp_req_func.line("Vec::new()");
    }

    fn gen_overwrittes(&self, scope: &mut Scope, _target: ast::AstTarget) {
        let func = scope.new_fn("overwrittes") 
            .arg("node", "&dag::DagNode")
            .ret("Vec<Reg>");

        for pat in &self.patterns {
            if pat.overwrittes.is_empty() { continue; }
            func.line(format!("{} {{", self.construct_cond(pat)));
            
            func.line("\tlet mut overwrittes = Vec::new();");
            for overwrite in &pat.overwrittes {
                let overwrite = overwrite.replace("\r", "");
                func.line(format!("\toverwrittes.push(Reg::{overwrite});"));
            }
            func.line("\treturn overwrittes;");

            func.line("}");
        }

        func.line("Vec::new()");
    }
}

fn first_to_uppercase(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn construct_asm(target: ast::AstTarget, pattern: &ast::Pattern, code: &mut Vec<String>, mut close: usize, tabs: fn(usize) -> String) {
    // construct operand generation code

    if let Some(_ls) = pattern.variant.ls {
        code.push(format!("{}let ls = {{", tabs(close)));
        close += 1;

        code.push(format!("{}let mut consta = None;", tabs(close)));
        code.push(format!("{}if OperationHandler::new().requires_new_const(&node.get_op(0)) {{ consta = Some(OperationHandler::new().create_const(module)) }}", tabs(close)));

        code.push(format!("{}if OperationHandler::new().just_op(&node.get_op(0)) {{", tabs(close)));
        close += 1;
        code.push(format!("{}OperationHandler::new().compile_op(&node.get_op(0), consta.as_ref()).unwrap()", tabs(close)));
        close -= 1;
        code.push(format!("{}}} else if OperationHandler::new().inserts_instrs(&node.get_op(0)) {{", tabs(close)));
        close += 1;
        code.push(format!("{}let Some(instrs) = OperationHandler::new().compile_instrs(&node.get_op(0), consta.as_ref(), DagTmpInfo::new(0xF0, node.get_ty())) else {{ panic!() }};", tabs(close)));
        code.push(format!("{}asm.extend_from_slice(&instrs);", tabs(close)));
        code.push(format!("{}Operand::Tmp(0xF0)", tabs(close)));
        close -= 1;
        code.push(format!("{}}} else {{ panic!() }}", tabs(close)));
        close -= 1;
        code.push(format!("{}}};", tabs(close)));
    }

    if let Some(_rs) = pattern.variant.rs {
        code.push(format!("{}let rs = {{", tabs(close)));
        close += 1;

        code.push(format!("{}let mut consta = None;", tabs(close)));
        code.push(format!("{}if OperationHandler::new().requires_new_const(&node.get_op(1)) {{ consta = Some(OperationHandler::new().create_const(module)) }}", tabs(close)));

        code.push(format!("{}if OperationHandler::new().just_op(&node.get_op(1)) {{", tabs(close)));
        close += 1;
        code.push(format!("{}OperationHandler::new().compile_op(&node.get_op(1), consta.as_ref()).unwrap()", tabs(close)));
        close -= 1;
        code.push(format!("{}}} else if OperationHandler::new().inserts_instrs(&node.get_op(1)) {{", tabs(close)));
        close += 1;
        code.push(format!("{}let Some(instrs) = OperationHandler::new().compile_instrs(&node.get_op(1), consta.as_ref(), DagTmpInfo::new(0xF1, node.get_ty())) else {{ panic!() }};", tabs(close)));
        code.push(format!("{}asm.extend_from_slice(&instrs);", tabs(close)));
        code.push(format!("{}Operand::Tmp(0xF1)", tabs(close)));
        close -= 1;
        code.push(format!("{}}} else {{ panic!() }}", tabs(close)));
        close -= 1;
        code.push(format!("{}}};", tabs(close)));
    }

    if let Some(_op3) = pattern.variant.op3 {
        code.push(format!("{}let op3 = {{", tabs(close)));
        close += 1;

        code.push(format!("{}let mut consta = None;", tabs(close)));
        code.push(format!("{}if OperationHandler::new().requires_new_const(&node.get_op(2)) {{ consta = Some(OperationHandler::new().create_const(module)) }}", tabs(close)));

        code.push(format!("{}if OperationHandler::new().just_op(&node.get_op(2)) {{", tabs(close)));
        close += 1;
        code.push(format!("{}OperationHandler::new().compile_op(&node.get_op(2), consta.as_ref()).unwrap()", tabs(close)));
        close -= 1;
        code.push(format!("{}}} else if OperationHandler::new().inserts_instrs(&node.get_op(2)) {{", tabs(close)));
        close += 1;
        code.push(format!("{}let Some(instrs) = OperationHandler::new().compile_instrs(&node.get_op(2), consta.as_ref(), DagTmpInfo::new(0xF2, node.get_ty())) else {{ panic!() }};", tabs(close)));
        code.push(format!("{}asm.extend_from_slice(&instrs);", tabs(close)));
        code.push(format!("{}Operand::Tmp(0xF2)", tabs(close)));
        close -= 1;
        code.push(format!("{}}} else {{ panic!() }}", tabs(close)));
        close -= 1;
        code.push(format!("{}}};", tabs(close)));
    }

    // construct assembly build

    for line in &pattern.lines {
        match line {
            AsmLine::Rust(rust) => code.push(rust.to_string()),
            AsmLine::Asm(asm) => code.push(format!("asm.push({});", construct_assembly_build(target, asm.replace("\n", "")))),
        }
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
    let arg_string = arg_string.replace("$1", "ls");
    let arg_string = arg_string.replace("$2", "rs");
    let arg_string = arg_string.replace("$3", "op3");
    
    let arg_string = arg_string.replace("%t0", &format!("Operand::Tmp(0)"));
    let arg_string = arg_string.replace("%t1", &format!("Operand::Tmp(1)"));
    let arg_string = arg_string.replace("%t2", &format!("Operand::Tmp(2)"));
    
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