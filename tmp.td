def Pat<Div .., .. -> ..> int 
    overwrite rcx
    overwrite rax
{
    asm -> mov Operand::Reg(Reg::Rcx()), 0;
}