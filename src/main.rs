#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;

use usb_device::{
    device::{
        UsbDevice,
        UsbDeviceState,
    },
    bus::UsbBusAllocator,
    prelude::*,
};

use usbd_hid::{
    hid_class::HIDClass,
    descriptor::{
        MediaKeyboardReport,
        generator_prelude::*,
    },
};

use embedded_hal as hal;

#[allow(dead_code)]
mod pac;
mod gpio;
mod wdog;
mod rtc;
mod dsp_filter;
mod capsense;

pub use efm32hg_usbd::{
    USBCore as Efm32USB,
    UsbBusType,
};

use crate::gpio::{
    GpioExt as _,
    OutputPin as _,
    ToggleableOutputPin as _,
    Output,
    OpenDrain,
    PA0,
    PB7,
};

const CPU_FREQUENCY_HZ: usize = 1_500_000;

fn cmu_periph_setup(p: &pac::Peripherals) {
    p.CMU.ushfrcoconf.write(|w| w.band()._48mhz());
    p.CMU.oscencmd.write(|w| w.ushfrcoen().set_bit());
    p.CMU.cmd.write(|w| w.hfclksel().ushfrcodiv2());

    // set CPU speed to 24MHz / 16 = 1.5MHz
    p.CMU.hfcoreclkdiv.write(|w| w.hfcoreclkdiv().hfclk16());

    //HFPERCLK should be 24MHz / 512 = 46875Hz
    p.CMU.hfperclkdiv.write(|w|
        w.hfperclken().set_bit()
         .hfperclkdiv().hfclk512()
    );
    p.CMU.hfperclken0.write(|w| w.gpio().set_bit());
}

fn wdog_set(p: &pac::Peripherals, value: u32) {
    #[allow(unsafe_code)]
    p.WDOG.ctrl.write(|w| unsafe { w.bits(value) })
}

#[allow(unsafe_code)]
fn rtc_setup(dp: &pac::Peripherals) {

    dp.CMU.hfcoreclken0.write(|w| w.le().set_bit());
    dp.CMU.oscencmd.write(|w| w.lfrcoen().set_bit());
    dp.CMU.lfapresc0.reset();
    dp.CMU.lfclksel.write(|w| w.lfa().lfrco());
    dp.CMU.lfaclken0.write(|w| w.rtc().set_bit());


    dp.RTC.freeze.reset();
    dp.RTC.ctrl.reset();
    dp.RTC.ien.reset();
    dp.RTC.ifc.write(|w| w.comp0().set_bit()
                          .comp1().set_bit()
                          .of().set_bit()
    );
    dp.RTC.comp0.reset();
    dp.RTC.comp1.reset();

    dp.RTC.comp0.write(|w| unsafe { w.comp0().bits(65_536) });
    dp.RTC.ien.modify(|_, w| w.comp0().set_bit());

    dp.RTC.ctrl.modify(|_, w| w.comp0top().set_bit());

    dp.RTC.ctrl.modify(|_, w| w.en().set_bit());

}

#[allow(unsafe_code)]
fn timer0_setup(dp: &pac::Peripherals) {

    dp.CMU.hfperclken0.modify(|_, w| w.timer0().set_bit());
    dp.TIMER0.ien.write(|w| w.of().set_bit());
    dp.TIMER0.top.write(|w| unsafe { w.bits(24) }); // ~500 usec
    dp.TIMER0.cmd.write(|w| w.start().set_bit());

}

const TOMU_VID: u16 = 0x1209;
const TOMU_PID: u16 = 0x70b1;

#[rtic::app(
    device = crate::pac,
    peripherals = true,
)]
const APP: () = {

    struct Resources {
        green: PA0<Output<OpenDrain>>,
        red: PB7<Output<OpenDrain>>,
        usb_dev: UsbDevice<'static, UsbBusType>,
        hid: HIDClass<'static, UsbBusType>,
        rtc: pac::RTC,
        usb: pac::USB,
        timer0: pac::TIMER0,
    }

    #[init]
    fn init(cx: init::Context) -> init::LateResources {

        let dp = cx.device;

        cmu_periph_setup(&dp);
        wdog_set(&dp, 0);
        rtc_setup(&dp);
        timer0_setup(&dp);

        let gpio = dp.GPIO.split();

        let mut green = gpio.pa0.into_open_drain_output();
        let mut red = gpio.pb7.into_open_drain_output();
        red.set_low().ok();
        green.set_high().ok();

        static mut USB_BUS: Option<UsbBusAllocator<UsbBusType>> = None;

        #[allow(unsafe_code)]
        unsafe {
            static mut USB_RX_BUFFER: [u32; 32] = [0; 32];
            let bus = UsbBusAllocator::new(Efm32USB::new(&mut USB_RX_BUFFER));
            USB_BUS = Some(bus);
        }

        #[allow(unsafe_code)]
        let hid = HIDClass::new(
            unsafe { USB_BUS.as_ref().unwrap() },
            MediaKeyboardReport::desc(),
            u8::MAX,
        );

        #[allow(unsafe_code)]
        let allocator = unsafe { USB_BUS.as_ref() }.unwrap();

        let mut usb_dev = UsbDeviceBuilder::new(
            allocator,
            UsbVidPid(TOMU_VID, TOMU_PID),
        )
            .manufacturer("Fake Company")
            .product("Media Key")
            .serial_number("TEST")
            .composite_with_iads()
            .max_packet_size_0(64)
            .build();

        usb_dev.force_reset().unwrap();

        init::LateResources {
            red,
            green,
            usb_dev,
            hid,
            rtc: dp.RTC,
            usb: dp.USB,
            timer0: dp.TIMER0,
        }

    }

    #[idle(resources = [red])]
    fn idle(ctx: idle::Context) -> ! {
        let red = ctx.resources.red;
        loop {
            // cortex_m::asm::wfi();
            cortex_m::asm::delay((CPU_FREQUENCY_HZ / 4) as u32);
            red.toggle().ok();
        }
    }

    #[task(binds = USB, resources = [usb_dev, hid], priority = 2)]
    fn usb_intr(ctx: usb_intr::Context) {
        let mut usb_dev = ctx.resources.usb_dev;
        let mut hid = ctx.resources.hid;

        usb_dev.lock(|usb_dev| {
            hid.lock(|hid| usb_dev.poll(&mut[hid]))
        });
    }

    #[task(binds = TIMER0, resources = [timer0], priority = 3)]
    fn timer0(ctx: timer0::Context) {
        let timer = ctx.resources.timer0;
        timer.ifc.write(|w| w.of().set_bit());
        rtic::pend(pac::Interrupt::USB);
    }

    #[task(binds = RTC, resources = [rtc, usb_dev, hid, green, usb], priority = 4)]
    fn rtc(ctx: rtc::Context) {
        static mut STATE: bool = false;
        let green = ctx.resources.green;
        let rtc = ctx.resources.rtc;
        let hid = ctx.resources.hid;
        let usb_state = ctx.resources.usb_dev.state();
        let usb = ctx.resources.usb;

        rtc.ifc.write(|w| w.comp0().set_bit());

        let _gintsts = usb.gintsts.read().bits();
        let _usb_if = usb.if_.read().bits();
        let _daint = usb.daint.read().bits();
        let _doep0int = usb.doep0int.read().bits();

        if usb_state != UsbDeviceState::Configured {
            green.set_low().ok();
            return;
        }

        let usage_id = if *STATE {
            0x00e2
        } else {
            0x0000
        };
        *STATE = !*STATE;
        green.set_high().ok();

        let report = MediaKeyboardReport { usage_id };
        hid.push_input(&report).ok();
        rtic::pend(pac::Interrupt::USB);
    }

};
