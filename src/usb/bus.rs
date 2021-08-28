pub use usb_device::bus::UsbBus;
use usb_device::{
    UsbDirection,
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
};

pub struct USB {
    _usb: pac::USB,
    // FIXME: Pin mode is probably irrelevant
    _pin_dm: PC14<Output<PushPull>>,
    _pin_dp: PC15<Output<PushPull>>,
}

impl USB {
    pub fn new(
        usb: pac::USB,
        pin_dm: PC14<Output<PushPull>>,
        pin_dp: PC15<Output<PushPull>>,
    ) -> UsbBusAllocator<Self> {
        let bus = Self {
            _usb: usb,
            _pin_dm: pin_dm,
            _pin_dp: pin_dp,
        };
        UsbBusAllocator::new(bus)
    }
}

#[allow(unsafe_code)]
unsafe impl Sync for USB {}

impl UsbBus for USB {

    fn alloc_ep(
        &mut self,
        ep_dir: UsbDirection,
        ep_addr: Option<EndpointAddress>,
        ep_type: EndpointType,
        max_packet_size: u16,
        interval: u8,
    ) -> UsbResult<EndpointAddress> {
        panic!("only control endpoint is supported");
    }

    fn enable(&mut self) { }

    fn reset(&self) { }

    fn set_device_address(&self, addr: u8) {
    }

    fn write(
        &self,
        ep_addr: EndpointAddress,
        buf: &[u8],
    ) -> UsbResult<usize> {
        Ok(buf.len())
    }

    fn read(
        &self,
        ep_addr: EndpointAddress,
        buf: &mut [u8],
    ) -> UsbResult<usize>
    {
        Ok(buf.len())
    }

    fn set_stalled(&self, ep_addr: EndpointAddress, stalled: bool) { }

    fn is_stalled(&self, ep_addr: EndpointAddress) -> bool {
        true
    }

    fn suspend(&self) { }

    fn resume(&self) { }

    fn poll(&self) -> PollResult {
        PollResult::None
    }

}
