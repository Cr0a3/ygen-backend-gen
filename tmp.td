def Pat<Add gr, gr -> gr> {
    asm -> lea $out, [$1 + $2]
}

def Pat<Add gr, imm -> gr> {
    if $out == $gr {
        asm -> add $out, $2
    } else {
        asm -> mov $out, $2
        asm -> add $out, $1
    }
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
}