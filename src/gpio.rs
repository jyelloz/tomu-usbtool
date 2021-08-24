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

impl GpioExt for pac::GPIO {
    type Parts = Parts;

    fn split(self) -> Self::Parts {
        Self::Parts {
            pa0: PA0 {
                i: 0,
                _mode: PhantomData,
            },
            pb7: PB7 {
                i: 7,
                _mode: PhantomData,
            },
        }
    }
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

pub struct PA0<MODE> {
    i: u8,
    _mode: PhantomData<MODE>,
}

impl <MODE> OutputPin for PA0<Output<MODE>> {
    type Error = Infallible;
    fn set_low(&mut self) -> Result<(), Self::Error> {
        #[allow(unsafe_code)]
        unsafe {
            (*pac::GPIO::ptr()).pa_doutset.write(|w| w.bits(1 << self.i))
        };
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Self::Error> {
        #[allow(unsafe_code)]
        unsafe {
            (*pac::GPIO::ptr()).pa_doutclr.write(|w| w.bits(1 << self.i))
        };
        Ok(())
    }
}

pub struct PB7<MODE> {
    i: u8,
    _mode: PhantomData<MODE>,
}

impl <MODE> OutputPin for PB7<Output<MODE>> {
    type Error = Infallible;
    fn set_low(&mut self) -> Result<(), Self::Error> {
        #[allow(unsafe_code)]
        unsafe {
            (*pac::GPIO::ptr()).pb_doutset.write(|w| w.bits(1 << self.i))
        };
        Ok(())
    }
    fn set_high(&mut self) -> Result<(), Self::Error> {
        #[allow(unsafe_code)]
        unsafe {
            (*pac::GPIO::ptr()).pb_doutclr.write(|w| w.bits(1 << self.i))
        };
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum Edge {
    Rising,
    Falling,
    RisingFalling,
}

pub struct Parts {
    pub pa0: PA0<Output<PushPull>>,
    pub pb7: PB7<Output<PushPull>>,
}
