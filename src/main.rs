#![no_main]
#![no_std]

use panic_rtt_target as _;
use rtt_target::{rtt_init_print, rprintln};

use cortex_m_rt::entry;
use microbit::{
    board::Board,
    hal::{Timer, twim, gpiote::Gpiote, delay::Delay, prelude::*},
    pac::{self, twim0::frequency::FREQUENCY_A, interrupt},
};
use cortex_m::asm;

use lsm303agr::*;

use critical_section_lock_mut::LockMut;

static P_GPIOTE: LockMut<Gpiote> = LockMut::new();

#[interrupt]
fn GPIOTE() {
    rprintln!("gpiote interrupt");
    P_GPIOTE.with_lock(|gpiote| gpiote.channel0().reset_events());
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
    let interrupt_pin = board.pins.p0_25.into_pullup_input();
    
    // On boot, the MB2 Interface MCU pulls the I2C_INT_INT
    // line low.  The line will remain pulled low until a
    // User Event message is read from the internal I2C bus.
    // This prevents any IMU interrupts from being seen.
    // The line will continue low for up to a second after
    // reading the User Event message.
    //
    // Thanks to Robert Elia, Elliot Roberts et al for this
    // code.
    rprintln!("clearing i2c_int_int");
    // Endpoint 0x70 is the IMCU. The User Event message is
    // 4 bytes. The Busy message is 2 bytes.
    let mut delay = Delay::new(board.SYST);
    delay.delay_ms(1000_u16);
    loop {
        if interrupt_pin.is_high().unwrap() {
            break;
        }
        let mut buf = [0u8; 255];
        i2c.read(0x70, &mut buf).unwrap();
        match &buf {
            &[0x20, 0x39, 0x0, ..] => {
                rprintln!("got 'busy' message");
                delay.delay_ms(1000_u16);
            }
            &[0x11, 0x09, 0x1, cause, ..] => {
                if !(1..=3).contains(&cause) {
                    panic!("unexpected 'user event' cause {}", cause);
                }
                rprintln!("got 'user event' message {}", cause);
                break;
            }
            _ => panic!("unexpected message {:x?}", buf),
        }
    }
    let mut msecs = 0;
    while interrupt_pin.is_low().unwrap() {
        if msecs >= 5000 {
            panic!("interrupt pin stuck low for {} ms", msecs);
        }
        delay.delay_ms(1u16);
        msecs += 1;
    }
    rprintln!("interrupt went high in {}ms", msecs);

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
        .input_pin(&interrupt_pin.degrade())
        .hi_to_lo()
        .enable_interrupt();
    channel0.reset_events();

    P_GPIOTE.init(gpiote);

    unsafe { pac::NVIC::unmask(pac::Interrupt::GPIOTE) };
    pac::NVIC::unpend(pac::Interrupt::GPIOTE);

    rprintln!("setup complete");

    lsm303.acc_enable_interrupt(Interrupt::DataReady1).unwrap();

    loop {
        asm::wfe();
    }
}
