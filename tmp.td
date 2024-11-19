def Pat<Add .., .. -> ..> int 
    map %t1, gr
{
    asm -> mov %t1, $1;
    asm -> add %t1, $2;
    asm -> mov $out, %t1;
}