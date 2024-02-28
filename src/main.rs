#![no_main]
#![no_std]

use panic_rtt_target as _;
use rtt_target::{rtt_init_print, rprintln};

use cortex_m_rt::entry;
use microbit::{
    board::Board,
    hal::{Timer, twim, gpiote::Gpiote, delay::Delay, prelude::*},
    pac::{twim0::frequency::FREQUENCY_A, interrupt},
};

use lsm303agr::*;

#[interrupt]
fn GPIOTE() {
    rprintln!("gpiote interrupt");
}

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();
    let mut i2c = twim::Twim::new(
        board.TWIM0,
        board.i2c_internal.into(),
        FREQUENCY_A::K100,
    );
    let mut timer = Timer::new(board.TIMER0);
    
    rprintln!("clearing i2c_int_int");
    // Thanks to Robert Elia et al for this code.
    let mut delay = Delay::new(board.SYST);
    let mut buf = [0u8; 5];
    let _ = i2c.read(0x70, &mut buf);
    delay.delay_ms(1000u16);

    rprintln!("continuing setup");

    let mut lsm303 = Lsm303agr::new_with_i2c(i2c);
    lsm303.init().unwrap();
    lsm303.set_accel_mode_and_odr(
        &mut timer,
        lsm303agr::AccelMode::Normal,
        lsm303agr::AccelOutputDataRate::Hz1,
    ).unwrap();
    
    let gpiote = Gpiote::new(board.GPIOTE);

    let channel0 = gpiote.channel0();
    channel0
        .input_pin(&board.pins.p0_25.into_floating_input().degrade())
        .hi_to_lo()
        .enable_interrupt();
    channel0.reset_events();

    rprintln!("setup complete");

    

    loop { }
}
