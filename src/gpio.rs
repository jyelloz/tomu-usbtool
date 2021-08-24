use core::{
    convert::Infallible,
    marker::PhantomData,
};

use crate::pac;

pub use crate::hal::digital::v2::OutputPin;

pub trait GpioExt {
    type Parts;

    fn split(self) -> Self::Parts;
}

pub struct Floating;
pub struct PullDown;
pub struct PullUp;
pub struct OpenDrain;
pub struct PushPull;

pub struct Input<MODE> {
    _mode: PhantomData<MODE>,
}

pub struct Output<MODE> {
    _mode: PhantomData<MODE>,
}

#[derive(Debug, PartialEq)]
pub enum Edge {
    Rising,
    Falling,
    RisingFalling,
}

macro_rules! gpio {
    ($GPIO:ident, $gpiox:ident, $PXx:ident, [
        $($PXi:ident: ($pxi:ident, $i:expr, $MODE:ty, $set:ident, $clr:ident),)+
    ]) => {
        pub struct Parts {
            $(
                pub $pxi: $PXi<$MODE>,
            )+
        }

        impl GpioExt for pac::GPIO {
            type Parts = Parts;

            fn split(self) -> Self::Parts {
                Self::Parts {
                    $(
                        $pxi: $PXi { _mode: PhantomData },
                    )+
                }
            }
        }

        $(
            pub struct $PXi<MODE> {
                _mode: PhantomData<MODE>,
            }
            impl <MODE> OutputPin for $PXi<Output<MODE>> {
                type Error = Infallible;
                fn set_low(&mut self) -> Result<(), Self::Error> {
                    #[allow(unsafe_code)]
                    unsafe {
                        (*pac::GPIO::ptr()).$set.write(|w| w.bits(1 << $i))
                    };
                    Ok(())
                }
                fn set_high(&mut self) -> Result<(), Self::Error> {
                    #[allow(unsafe_code)]
                    unsafe {
                        (*pac::GPIO::ptr()).$clr.write(|w| w.bits(1 << $i))
                    };
                    Ok(())
                }
            }

        )+
    }
}

gpio!(GPIO, gpio, PXx, [
    PA0: (pa0, 0, Output<PushPull>, pa_doutset, pa_doutclr),
    PB7: (pb7, 7, Output<PushPull>, pb_doutset, pb_doutclr),
]);
