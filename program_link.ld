ENTRY(_start)
SECTIONS
{
    . = 0x20000000000;
    .text :
    {
        *(.text)
    }
    .data :
    {
        *(.data)
    }
    .bss :
    {
        *(.bss)
    }
    /DISCARD/ : { *(.comment) *(.eh_frame) }
}