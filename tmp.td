def Pat<Ret> {
    asm -> ret;
}

def Pat<Add gr, gr -> gr> {
    asm -> lea $out, [$1 + $2];
}

def Pat<Add gr, imm -> gr> {
    asm -> mov $out, $2;
    asm -> add $out, $1;
}
