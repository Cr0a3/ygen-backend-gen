def Pat<Div .., .. -> ..> int 
    overwrite rcx;
    map %t1, gr;
    overwrite rax;
{
    asm -> mov Operand::Reg(Reg::Rcx()), 0;
}