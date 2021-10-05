use crate::pac;

/// Watchdog Timer
pub struct Dog(pub pac::WDOG);

impl Dog {

    pub fn disable(&self) {
        self.0.ctrl.modify(|_, w| w.en().clear_bit());
    }

    pub fn enable(&self) {
        self.0.ctrl.modify(|_, w| w.en().set_bit());
    }

    /// Assert positive control over the device to defer failure.
    pub fn feed(&self) {
        self.0.cmd.write(|w| w.clear().set_bit());
    }

    /// Deconfigure and take back the underlying peripheral.
    pub fn take(self) -> pac::WDOG {
        self.feed();
        self.disable();
        self.0
    }

}
