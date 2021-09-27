set confirm off
target extended-remote | openocd -c 'gdb_port pipe; log_output openocd.log' -f openocd.cfg

set print asm-demangle on

set pagination off

# set backtrace limit to not have infinite backtrace loops
set backtrace limit 32

# detect unhandled exceptions, hard faults and panics
break DefaultHandler
break HardFault
break rust_begin_unwind

# break main

monitor arm semihosting enable
monitor arm semihosting_fileio enable

monitor itm port 0 on

# set $usb       = 0x4003c000
# set $dwc       = 0x40100000
# set $dwc_fifo0 = 0x40101000
# set $dwc_fifo1 = 0x40101000
# set $dwc_fifo2 = 0x40102000
# set $dwc_fifo3 = 0x40103000

load

# start the process but immediately halt the processor
c

# vim: ft=gdb :
