use crate::{
    pac::{
        self,
        SynopsysUSB,
    },
    gpio::{
        PC14,
        PC15,
        Output,
        PushPull,
    },
};

// use synopsys_usb_otg::UsbPeripheral;
// pub use synopsys_usb_otg::UsbBus;
// pub use synopsys_usb_otg::efm32::UsbBus;

pub struct USB {
    pub usb: pac::USB,
    // TODO: ensure the pin modes are correct.
    pub pin_dm: PC14<Output<PushPull>>,
    pub pin_dp: PC15<Output<PushPull>>,
}

#[allow(unsafe_code)]
unsafe impl Sync for USB {}

#[allow(unsafe_code)]
unsafe impl UsbPeripheral for USB {

    const REGISTERS: *const () = pac::USB::OTG_BASE;

    const HIGH_SPEED: bool = false;

    const FIFO_DEPTH_WORDS: usize = 256;

    const ENDPOINT_COUNT: usize = 3;

    #[allow(unsafe_code)]
    fn enable() {

        let cmu = unsafe { &*pac::CMU::ptr() };
        let usb = unsafe { &*pac::USB::ptr() };

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

    fn ahb_frequency_hz(&self) -> u32 {
        14_000_000
    }

}

pub type UsbBusType = UsbBus<USB>;
