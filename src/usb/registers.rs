pub const FIFO_LEN_BYTES: usize = 1536;
pub const FIFO_LEN_ITEMS: usize = FIFO_LEN_BYTES >> 2;

#[link_section=".usbfifo.ep0"]
pub static mut USB_FIFO0: [u32; FIFO_LEN_ITEMS] = [0; FIFO_LEN_ITEMS];
#[link_section=".usbfifo.ep1"]
pub static mut USB_FIFO1: [u32; FIFO_LEN_ITEMS] = [0; FIFO_LEN_ITEMS];
#[link_section=".usbfifo.ep2"]
pub static mut USB_FIFO2: [u32; FIFO_LEN_ITEMS] = [0; FIFO_LEN_ITEMS];
#[link_section=".usbfifo.ep3"]
pub static mut USB_FIFO3: [u32; FIFO_LEN_ITEMS] = [0; FIFO_LEN_ITEMS];
