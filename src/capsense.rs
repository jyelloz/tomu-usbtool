use heapless::spsc::{
    Queue,
    Consumer,
    Producer,
};

use crate::{
    pac,
    dsp_filter::{
        Max,
        MovingAverage,
    },
};

const CHANNEL_COUNT: usize = 2;
const CAPSENSE_QUEUE_DEPTH: usize = 4;
pub type CapsenseQueue = Queue<CapsenseData, CAPSENSE_QUEUE_DEPTH>;
pub struct CapsenseData(pub [CapsenseChannelData; CHANNEL_COUNT]);
pub struct CapsenseRead(pub Consumer<'static, CapsenseData, CAPSENSE_QUEUE_DEPTH>);
pub struct CapsenseWrite(pub Producer<'static, CapsenseData, CAPSENSE_QUEUE_DEPTH>);

#[derive(Clone, Copy)]
pub enum CapsenseChannel {
    A,
    B,
}

impl CapsenseChannel {
    pub fn next(&self) -> Self {
        use CapsenseChannel::*;
        match *self {
            A => B,
            B => A,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CapsenseChannelData {
    average: MovingAverage,
    max: Max,
}

impl CapsenseChannelData {
    pub fn push(&mut self, value: u16) {
        self.max.process(value);
        self.average.process(value);
    }
    pub fn max(&self) -> u16 {
        self.max.current()
    }
    pub fn moving_average(&self) -> u16 {
        self.average.latest_filtered_value()
    }
}

impl Default for CapsenseChannelData {
    fn default() -> Self {
        Self {
            average: MovingAverage::default(),
            max: Max::default(),
        }
    }
}

pub struct Capsense {
    acmp: pac::ACMP0,
    timer0: pac::TIMER0,
    timer1: pac::TIMER1,
    tx: CapsenseWrite,
    active_channel: CapsenseChannel,
    channels: [CapsenseChannelData; CHANNEL_COUNT],
}

impl CapsenseRead {
    pub fn read(&mut self) -> Option<CapsenseData> {
        self.0.dequeue()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl CapsenseWrite {
    pub fn write(&mut self, data: CapsenseData) -> Result<(), CapsenseData> {
        self.0.enqueue(data)
    }
}

impl Capsense {

    pub fn new(
        acmp: pac::ACMP0,
        timer0: pac::TIMER0,
        timer1: pac::TIMER1,
        tx: CapsenseWrite,
    ) -> Self {
        Self {
            acmp,
            timer0,
            timer1,
            tx,
            active_channel: CapsenseChannel::A,
            channels: [CapsenseChannelData::default(); CHANNEL_COUNT],
        }
    }

    pub fn setup(
        &mut self,
        cmu: &pac::CMU,
        prs: &pac::PRS,
    ) {
        let Self { acmp, timer0, timer1, .. } = self;

        cmu.hfperclken0.modify(|_, w|
            w.timer0().set_bit()
             .timer1().set_bit()
             .prs().set_bit()
             .acmp0().set_bit()
        );

        acmp.ctrl.write(|w| unsafe {
            w.halfbias().clear_bit()
             .fullbias().clear_bit()
             .warmtime()._512cycles()
             .hystsel().hyst7()
             .biasprog().bits(7)
        });
        acmp.inputsel.write(|w| unsafe {
            w.csressel().res0()
             .csresen().set_bit()
             .lpref().clear_bit()
             .negsel().capsense()
             .vddlevel().bits(0x3d)
        });
        acmp.ctrl.modify(|_, w| w.en().set_bit());
        while !acmp.status.read().acmpact().bit_is_set() { }

        timer1.ctrl.write(|w|
            w.presc().div1024()
             .clksel().cc1()
        );
        timer1.top.reset();
        timer1.cc1_ctrl.write(|w|
            w.mode().inputcapture()
             .prssel().prsch0()
             .insel().set_bit()
             .icevctrl().rising()
             .icedge().both()
        );

        prs.ch0_ctrl.write(|w| unsafe {
            w.edsel().posedge()
             .sourcesel().acmp0()
             .sigsel().bits(0)
        });

        timer0.ctrl.write(|w| w.presc().div512());
        timer0.top.write(|w| unsafe { w.top().bits(10) });
        timer0.ien.write(|w| w.of().set_bit());

    }

    pub fn measure(&mut self) {
        use CapsenseChannel::*;

        let Self { acmp, timer0, timer1, active_channel, .. } = &self;

        match *active_channel {
            A => acmp.inputsel.modify(|_, w| w.possel().ch0()),
            B => acmp.inputsel.modify(|_, w| w.possel().ch1()),
        }

        timer0.cnt.reset();
        timer1.cnt.reset();

        timer0.cmd.write(|w| w.start().set_bit());
        timer1.cmd.write(|w| w.start().set_bit());

    }

    pub fn capture(&mut self) -> &[CapsenseChannelData] {
        let Self {
            timer0,
            timer1,
            channels,
            active_channel,
            tx,
            ..
        } = self;
        timer1.cmd.write(|w| w.stop().set_bit());
        timer0.cmd.write(|w| w.stop().set_bit());
        timer0.ifc.write(|w| w.of().set_bit());
        let bits = timer1.cnt.read().cnt().bits();
        let channel_data = &mut channels[*active_channel as usize];
        channel_data.push(bits);
        if let CapsenseChannel::B = *active_channel {
            tx.write(CapsenseData(*channels)).ok();
        }
        *active_channel = active_channel.next();
        channels
    }

}
