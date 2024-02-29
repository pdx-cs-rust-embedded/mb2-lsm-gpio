#![no_main]
#![no_std]

use cortex_m_rt::entry;
use microbit::{
    board::Board,
    hal::{Timer, Delay, twim, gpiote::Gpiote, gpio::{Level, Pin, Output, PushPull}, prelude::*},
    pac::{self, twim0::frequency::FREQUENCY_A, interrupt},
};
use cortex_m::asm;

use lsm303agr::*;

use critical_section_lock_mut::LockMut;

static P_GPIOTE: LockMut<Gpiote> = LockMut::new();

#[interrupt]
fn GPIOTE() {
    P_GPIOTE.with_lock(|gpiote| gpiote.channel0().reset_events());
}

static PANIC_LED: LockMut<Pin<Output<PushPull>>> = LockMut::new();

#[entry]
fn main() -> ! {
    let board = Board::take().unwrap();
    let mut i2c = twim::Twim::new(
        board.TWIM0,
        board.i2c_internal.into(),
        FREQUENCY_A::K100,
    );
    let mut timer = Timer::new(board.TIMER0);
    let interrupt_pin = board.pins.p0_25.into_pullup_input();
    let mut delay = Delay::new(board.SYST);
    
    let _row1 = board.display_pins.row1.into_push_pull_output(Level::High);
    let _on_led = board.display_pins.col1.into_push_pull_output(Level::Low);
    let mut read_led = board.display_pins.col2.into_push_pull_output(Level::High);
    let mut busy_led = board.display_pins.col3.into_push_pull_output(Level::High);
    let mut ready_led = board.display_pins.col4.into_push_pull_output(Level::High);
    let panic_led = board.display_pins.col5.into_push_pull_output(Level::High);
    PANIC_LED.init(panic_led.degrade());

    // On boot, the MB2 Interface MCU pulls the I2C_INT_INT
    // line low.  The line will remain pulled low until a
    // User Event message is read from the internal I2C bus.
    // This prevents any IMU interrupts from being seen.
    // The line may continue low for a while after
    // reading the User Event message.
    //
    // XXX Right now, the IMCU may decide that it is "busy"
    // and respond to reads with a Busy error response and
    // not release the interrupt line. Analysis is in
    // progress.
    //
    // Thanks to Robert Elia, Elliot Roberts et al for this
    // code.
    loop {
        delay.delay_ms(100u16);
        if interrupt_pin.is_high().unwrap() {
            break;
        }
        let buf = [0u8];
        i2c.write(0x70, &buf).unwrap();
        delay.delay_ms(100u16);
        // Endpoint 0x70 is the IMCU. The User Event message is
        // 4 bytes. The Busy message is 2 bytes.
        let mut buf = [0u8; 255];
        read_led.set_low().unwrap();
        i2c.read(0x70, &mut buf).unwrap();
        match buf {
            [0x20, 0x39, ..] => {
                busy_led.set_low().unwrap();
            }
            [0x11, 0x09, 0x1, cause, ..] => {
                if !(1..=3).contains(&cause) {
                    panic!();
                }
                break;
            }
            _ => panic!(),
        }
    }
    ready_led.set_low().unwrap();

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

    lsm303.acc_enable_interrupt(Interrupt::DataReady1).unwrap();

    loop {
        asm::wfe();
    }
}

#[panic_handler]
fn panic_handler(_: &core::panic::PanicInfo<'_>) -> ! {
    PANIC_LED.with_lock(|panic_led| {
        let _ = panic_led.set_high();
    });
    loop {
        asm::wfe();
    }
}
