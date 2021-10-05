use crate::pac;

/// Real Time Clock
pub struct Clock(pub pac::RTC);

impl Clock {

    pub fn enable(self, cmu: &pac::CMU) -> Self {

        cmu.hfcoreclken0.write(|w| w.le().set_bit());
        cmu.oscencmd.write(|w| w.lfrcoen().set_bit());
        cmu.lfapresc0.reset();
        cmu.lfclksel.write(|w| w.lfa().lfrco());
        cmu.lfaclken0.write(|w| w.rtc().set_bit());

        let Self(rtc) = &self;
        rtc.freeze.reset();
        rtc.ctrl.reset();
        rtc.ien.reset();
        rtc.ifc.write(|w| w.comp0().set_bit()
            .comp1().set_bit()
            .of().set_bit()
        );
        rtc.comp0.reset();
        rtc.comp1.reset();

        rtc.comp0.write(|w| unsafe { w.comp0().bits(65_536) });
        rtc.ien.modify(|_, w| w.comp0().set_bit());

        rtc.ctrl.modify(|_, w| w.comp0top().set_bit());

        rtc.ctrl.modify(|_, w| w.en().set_bit());

        self
    }

}
