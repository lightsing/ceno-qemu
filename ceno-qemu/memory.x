MEMORY
{
  RAM : ORIGIN = 0x80200000, LENGTH = 62M
  FLASH : ORIGIN = 0x20000000, LENGTH = 16M
}

REGION_ALIAS("REGION_TEXT", FLASH);
REGION_ALIAS("REGION_RODATA", FLASH);
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", RAM);

_heap_size = 54M;