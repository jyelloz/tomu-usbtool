MEMORY
{
  /* NOTE K = KiB = 1024 bytes */
  /* when using toboot, start at 0x4000 with 48K flash */
  FLASH : ORIGIN = 0x00004000, LENGTH = 0xC000
  /* when connected to debugger, use all 64K flash when necessary */
  /* FLASH : ORIGIN = 0x00000000, LENGTH = 0x10000 */
  /* FLASH : ORIGIN = 0x00000000, LENGTH = 0x0C000 */
  RAM : ORIGIN = 0x20000000, LENGTH = 0x2000
}

/* This is where the call stack will be allocated. */
/* The stack is of the full descending type. */
/* NOTE Do NOT modify `_stack_start` unless you know what you are doing */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);
