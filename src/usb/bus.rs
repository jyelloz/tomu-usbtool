use core::cell::Cell;
pub use usb_device::bus::UsbBus;
use core::convert::TryInto;
use usb_device::{
    UsbDirection::{self, *},
    UsbError,
    Result as UsbResult,
    bus::{
        UsbBusAllocator,
        PollResult,
    },
    endpoint::{
        EndpointAddress,
        EndpointType,
    },
};
use crate::{
    pac,
    gpio::{
        PC14,
        PC15,
        Output,
        PushPull,
    },
    usb::registers,
};

pub struct USB {
    usb: pac::USB,
    // FIXME: Pin mode is probably irrelevant
    _pin_dm: PC14<Output<PushPull>>,
    _pin_dp: PC15<Output<PushPull>>,
    in_endpoints: Endpoints,
    out_endpoints: Endpoints,
    in_ctrl: Option<ControlEndpoint>,
    out_ctrl: Option<ControlEndpoint>,
}

impl USB {
    pub fn new(
        usb: pac::USB,
        pin_dm: PC14<Output<PushPull>>,
        pin_dp: PC15<Output<PushPull>>,
    ) -> UsbBusAllocator<Self> {
        let bus = Self {
            usb,
            _pin_dm: pin_dm,
            _pin_dp: pin_dp,
            in_endpoints: Endpoints::new(UsbDirection::In),
            out_endpoints: Endpoints::new(UsbDirection::Out),
            in_ctrl: None,
            out_ctrl: None,
        };
        UsbBusAllocator::new(bus)
    }
}

impl USB {
    fn get_available_tx_space(&self, addr: EndpointAddress) -> usize {
        match addr.index() {
            0 => self.usb.diep0ctl.read().mps().bits() as usize,
            _ => 0,
        }
    }
    fn get_available_rx_space(&self, addr: EndpointAddress) -> usize {
        match addr.index() {
            0 => self.usb.doep0tsiz.read().xfersize().bits() as usize * 4,
            _ => 0,
        }
    }
}

#[allow(unsafe_code)]
impl USB {

    fn enable_phy(&mut self) {

        let cmu = unsafe { &*pac::CMU::ptr() };
        let usb = &self.usb;

        // Switch on clocks
        cmu.hfcoreclken0.modify(
            |_, w| w.usb().set_bit()
                    .usbc().set_bit()
                    .le().set_bit()
        );

        // Choose LFRC as LFC Clock
        cmu.lfclksel.write(|w| w.lfc().lfrco());
        // Enable USBLE
        cmu.lfcclken0.modify(|_, w| w.usble().set_bit());
        // Calibrate
        cmu.ushfrcoconf.write(|w| w.band()._48mhz());
        // Clock recovery
        cmu.usbcrctrl.modify(|_, w| w.en().set_bit());

        // Enable clock
        cmu.oscencmd.write(|w| w.ushfrcoen().set_bit());
        while !cmu.status.read().ushfrcordy().bit_is_set() { }

        // Select clock
        cmu.cmd.write(|w| w.usbcclksel().ushfrco());
        while !cmu.status.read().usbcushfrcosel().bit_is_set() { }

        // Turn off low energy mode features.
        usb.ctrl.write(|w| unsafe { w.bits(0) });

        // Switch on PHY.
        usb.route.write(|w| w.phypen().set_bit());
    }

    fn wait_for_idle(&self) {
        while self.usb.grstctl.read().ahbidle().bit_is_clear() {}
    }

    fn soft_reset(&self) {
        self.usb.grstctl.modify(|_, w| w.csftrst().set_bit());
        while self.usb.grstctl.read().csftrst().bit_is_set() {}
    }

    fn set_turnaround_time(&self) {
        self.usb.gusbcfg.write(|w| unsafe { w.usbtrdtim().bits(5) });
    }

    fn enable_dwc(&self) {

        let usb = &self.usb;

        // Wait for AHB ready
        self.wait_for_idle();

        // Configure OTG as device
        self.set_turnaround_time();

        // Perform core soft-reset
        self.wait_for_idle();
        self.soft_reset();

        // Activate the USB Transceiver
        // usb.gccfg.modify(|_, w| w.powerdown().set_bit());

        // Enable PHY clock
        // usb.pcgcctl.write(|w| unsafe { w.bits(0) });
        usb.pcgcctl.reset();

        // Soft disconnect device
        usb.dctl.modify(|_, w| w.sftdiscon().set_bit());

        // Setup USB speed and frame interval
        usb.dcfg.modify(|_, w| w.devspd().fs());

        // unmask EP interrupts
        usb.diepmsk.write(|w| w.xfercomplmsk().set_bit());

        // unmask core interrupts
        usb.gintmsk.write(|w| w.usbrstmsk().set_bit()
            .enumdonemsk().set_bit()
            .usbsuspmsk().set_bit()
            .wkupintmsk().set_bit()
            .iepintmsk().set_bit()
            .rxflvlmsk().set_bit()
        );

        // clear pending interrupts
        usb.gintsts.write(|w| unsafe { w.bits(0xffffffff) });

        // unmask global interrupt
        usb.gahbcfg.modify(|_, w| w.glblintrmsk().set_bit());

        // connect(true)
        usb.dctl.modify(|_, w| w.sftdiscon().clear_bit());
    }

    fn allocate_control_endpoint(
        &mut self,
        dir: UsbDirection,
    ) -> UsbResult<EndpointAddress> {
        match dir {
            In => if self.in_ctrl.is_some() {
                Err(UsbError::InvalidEndpoint)
            } else {
                let addr = EndpointAddress::from_parts(0, dir);
                self.in_ctrl = Some(ControlEndpoint::new());
                Ok(addr)
            },
            Out => if self.out_ctrl.is_some() {
                Err(UsbError::InvalidEndpoint)
            } else {
                let addr = EndpointAddress::from_parts(0, dir);
                self.out_ctrl = Some(ControlEndpoint::new());
                Ok(addr)
            },
        }
    }

    fn allocate_interface_endpoint(
        &mut self,
        dir: UsbDirection,
    ) -> UsbResult<EndpointAddress> {
        match dir {
            In => self.in_endpoints.allocate(),
            Out => self.out_endpoints.allocate(),
        }
    }

    fn control_tx(&self, buf: &[u8]) -> UsbResult<usize> {
        let endpoint = self.in_ctrl.as_ref().ok_or(UsbError::InvalidEndpoint)?;
        let len_bytes = buf.len().min(registers::FIFO_LEN_BYTES);
        let len_words = len_bytes >> 2;
        let extra_bytes = len_bytes & 0b11;
        if len_bytes < 1 {
            return Err(UsbError::WouldBlock);
        }
        endpoint.pend();
        unsafe {
            // TODO: might need to do this in blocks of 4 bytes due to
            // endianness.
            for i in 0..len_words {
                let src_offset = i << 2;
                let (src_slice, _) = buf[src_offset..].split_at(core::mem::size_of::<u32>());
                registers::USB_FIFO0[i] = u32::from_ne_bytes(src_slice.try_into().unwrap());
            }
            if extra_bytes > 0 {
                let src = &buf[len_words << 2..];
                let mut final_word: [u8; 4] = [0; 4];
                match extra_bytes {
                    1 => {
                        final_word[0] = src[0];
                    },
                    2 => {
                        final_word[0] = src[0];
                        final_word[1] = src[1];
                    },
                    3 => {
                        final_word[0] = src[0];
                        final_word[1] = src[1];
                        final_word[2] = src[2];
                    },
                    _ => unreachable!(),
                }
                registers::USB_FIFO0[len_words] = u32::from_ne_bytes(final_word);
            }
        }
        Ok(len_bytes)
    }

    fn control_rx(&self, buf: &mut [u8]) -> UsbResult<usize> {
        // TODO: you need to know how many bytes the host has actually sent
        let len_bytes = buf.len().min(registers::FIFO_LEN_BYTES);
        let len_words = len_bytes >> 2;
        let extra_bytes = len_bytes & 0b11;
        unsafe {
            for i in 0..len_words {
                let dst_offset = i << 2;
                let (src_slice, _) = &registers::USB_FIFO0[i..]
                    .split_at(core::mem::size_of::<u32>());
                let (dest, _) = buf[dst_offset..].split_at_mut(core::mem::size_of::<u32>());
                let dest: &mut [u8; 4] = dest.try_into().unwrap();
                *dest = src_slice[0].to_ne_bytes();
            }
            if extra_bytes > 0 {
                let final_word = registers::USB_FIFO0[len_words - 1];
                let final_word = &final_word.to_ne_bytes()[..extra_bytes];
                let dst_base = &mut buf[len_words << 2..];
                match extra_bytes {
                    1 => {
                        dst_base[0] = final_word[0];
                    }
                    2 => {
                        dst_base[0] = final_word[0];
                        dst_base[1] = final_word[1];
                    }
                    3 => {
                        dst_base[0] = final_word[0];
                        dst_base[1] = final_word[1];
                        dst_base[2] = final_word[2];
                    }
                    _ => unreachable!(),
                }
            }
        }
        Ok(len_bytes)
    }

}

#[allow(unsafe_code)]
unsafe impl Sync for USB {}

impl UsbBus for USB {

    fn alloc_ep(
        &mut self,
        dir: UsbDirection,
        addr: Option<EndpointAddress>,
        _ep_type: EndpointType,
        _max_packet_size: u16,
        _interval: u8,
    ) -> UsbResult<EndpointAddress> {
        if let Some(0) = addr.map(|a| a.index()) {
            self.allocate_control_endpoint(dir)
        } else {
            self.allocate_interface_endpoint(dir)
        }
    }

    fn enable(&mut self) {
        self.enable_phy();
        self.enable_dwc()
    }

    fn reset(&self) { }

    fn set_device_address(&self, addr: u8) {
        #[allow(unsafe_code)]
        self.usb.dcfg.modify(|_, w| unsafe { w.devaddr().bits(addr) });
    }

    fn write(
        &self,
        addr: EndpointAddress,
        buf: &[u8],
    ) -> UsbResult<usize> {
        if !addr.is_in() || addr.index() > self.in_endpoints.len() {
            return Err(UsbError::InvalidEndpoint);
        }
        match addr.index() {
            0 => self.control_tx(&buf),
            _ => Err(UsbError::InvalidEndpoint),
        }
    }

    fn read(
        &self,
        addr: EndpointAddress,
        buf: &mut [u8],
    ) -> UsbResult<usize>
    {
        if !addr.is_out() || addr.index() > self.out_endpoints.len() {
            return Err(UsbError::InvalidEndpoint);
        }
        match addr.index() {
            0 => self.control_rx(buf),
            _ => Err(UsbError::InvalidEndpoint),
        }
    }

    fn set_stalled(&self, addr: EndpointAddress, stalled: bool) {
        match addr.direction() {
            In => match addr.index() {
                0 => self.usb.doep0ctl.modify(|_, w| w.stall().bit(stalled)),
                _ => {},
            },
            Out => match addr.index() {
                0 => self.usb.diep0ctl.modify(|_, w| w.stall().bit(stalled)),
                _ => {},
            },
        }
    }

    fn is_stalled(&self, addr: EndpointAddress) -> bool {
        match addr.direction() {
            In => match addr.index() {
                0 => self.usb.diep0ctl.read().stall().bit(),
                _ => true,
            },
            Out => match addr.index() {
                0 => self.usb.doep0ctl.read().stall().bit(),
                _ => true,
            },
        }
    }

    fn suspend(&self) { }

    fn resume(&self) { }

    fn poll(&self) -> PollResult {

        let reset = self.usb.gintsts.read().usbrst().bit_is_set();
        let enum_done = self.usb.gintsts.read().enumdone().bit_is_set();

        if reset {
            self.reset();
            return PollResult::Reset;
        }

        if enum_done {
            return PollResult::Reset;
        }

        if let Some(in_ctrl) = &self.in_ctrl {
            if in_ctrl.is_pending() {
                // notify host
            }
            in_ctrl.unpend();
        }

        PollResult::None
    }

    const QUIRK_SET_ADDRESS_BEFORE_STATUS: bool = true;

}

/// A Marker type to represent Endpoint 0.
struct ControlEndpoint {
    pending: Cell<bool>,
}

impl ControlEndpoint {
    fn new() -> Self {
        Self { pending: Cell::new(false) }
    }
    fn is_pending(&self) -> bool {
        self.pending.get()
    }
    fn pend(&self) {
        self.pending.set(true);
    }
    fn unpend(&self) {
        self.pending.set(false);
    }
}

/// A Stack to represent device Endpoints 1, 2, and 3 in a single Direction.
struct Endpoints {
    endpoints_allocated: usize,
    dir: UsbDirection,
}

impl Endpoints {

    // EFM32HG supports 3 endpoints.
    const CHIP_CAPACITY: usize = 3;


    /// Get an empty Stack of Endpoints for the supplied Direction.
    fn new(dir: UsbDirection) -> Self {
        Self {
            dir,
            endpoints_allocated: 0,
        }
    }

    /// How many Endpoints have been allocated.
    fn len(&self) -> usize {
        self.endpoints_allocated as usize
    }

    /// Always allocates the endpoints sequentially.
    fn allocate(&mut self) -> UsbResult<EndpointAddress> {
        let position = self.endpoints_allocated as usize;
        if position >= Self::CHIP_CAPACITY {
            return Err(UsbError::EndpointOverflow);
        }
        let position = position + 1;
        self.endpoints_allocated = position;
        Ok(EndpointAddress::from_parts(position, self.dir))
    }

    // TODO: Figure out how to deallocate.

}
