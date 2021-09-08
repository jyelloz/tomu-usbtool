use core::{
    convert::Infallible,
    marker::PhantomData,
};

use paste::paste;

use crate::pac;

pub use crate::hal::digital::v2::{
    OutputPin,
    ToggleableOutputPin,
};

pub trait GpioExt {
    type Parts;

    fn split(self) -> Self::Parts;
}

pub struct Disabled;
pub struct PushPull;
pub struct WiredAnd;

pub type OpenDrain = WiredAnd;

pub struct Input<MODE>(PhantomData<MODE>);

pub struct Output<MODE>(PhantomData<MODE>);

macro_rules! gpio {
    ($gpiox:ident, $PXx:ident, [
        $($PXi:ident: ($bank:ident, $i:expr, $mode_part:ident),)+
    ]) => {
        paste! {
            pub struct Parts {
                $(
                    pub [<$PXi:lower>]: $PXi<Input<Disabled>>,
                )+
            }

            impl GpioExt for pac::GPIO {
                type Parts = Parts;

                fn split(self) -> Self::Parts {
                    Self::Parts {
                        $(
                            [<$PXi:lower>]: $PXi(PhantomData),
                        )+
                    }
                }
            }

            $(
                pub struct $PXi<MODE>(PhantomData<MODE>);

                #[allow(dead_code)]
                impl <MODE> $PXi<MODE> {
                    pub fn into_disabled_input(self) -> $PXi<Input<Disabled>> {
                        #[allow(unsafe_code)]
                        let gpio = unsafe { &*pac::GPIO::ptr() };
                        paste! {
                            gpio.[<p $bank _mode $mode_part>]
                                .modify(|_, w| w.[<mode $i>]().disabled());
                        }
                        $PXi(PhantomData)
                    }
                    pub fn into_push_pull_output(self) -> $PXi<Output<PushPull>> {
                        #[allow(unsafe_code)]
                        let gpio = unsafe { &*pac::GPIO::ptr() };
                        paste! {
                            gpio.[<p $bank _mode $mode_part>]
                                .modify(|_, w| w.[<mode $i>]().pushpull());
                        }
                        $PXi(PhantomData)
                    }
                    pub fn into_open_drain_output(self) -> $PXi<Output<WiredAnd>> {
                        #[allow(unsafe_code)]
                        let gpio = unsafe { &*pac::GPIO::ptr() };
                        paste! {
                            gpio.[<p $bank _mode $mode_part>]
                                .modify(|_, w| w.[<mode $i>]().wiredand());
                        }
                        $PXi(PhantomData)
                    }
                }
                impl <MODE> OutputPin for $PXi<Output<MODE>> {
                    type Error = Infallible;
                    fn set_low(&mut self) -> Result<(), Self::Error> {
                        #[allow(unsafe_code)]
                        unsafe {
                            (*pac::GPIO::ptr()).[<p $bank _doutset>].write(|w| w.bits(1 << $i))
                        };
                        Ok(())
                    }
                    fn set_high(&mut self) -> Result<(), Self::Error> {
                        #[allow(unsafe_code)]
                        unsafe {
                            (*pac::GPIO::ptr()).[<p $bank _doutclr>].write(|w| w.bits(1 << $i))
                        };
                        Ok(())
                    }
                }
                impl <MODE> ToggleableOutputPin for $PXi<Output<MODE>> {
                    type Error = Infallible;
                    fn toggle(&mut self) -> Result<(), Self::Error> {
                        #[allow(unsafe_code)]
                        unsafe {
                            (*pac::GPIO::ptr()).[<p $bank _douttgl>].write(|w| w.bits(1 << $i))
                        };
                        Ok(())
                    }
                }
            )+
        }
    }
}

// PA0/PB7 = Green/Red Light
// PC0/PC1 = Cap0/1B
// PC14/15 = USB +/-
// PE12/13 = Cap0/1A
gpio!(gpio, PXx, [
    PA0: (a, 0, l),
    PB7: (b, 7, l),

    PC0: (c, 0, l),
    PC1: (c, 1, l),

    PC14: (c, 14, h),
    PC15: (c, 15, h),

    PE12: (e, 12, h),
    PE13: (e, 13, h),
]);
