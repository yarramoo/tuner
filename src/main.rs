#![no_std]
#![no_main]

use core::{ops::RangeInclusive, range::Range};

// use defmt_rtt as _;
use rtt_target::{rtt_init_print, rprintln};
use panic_rtt_target as _;
// use panic_halt as _;

use cortex_m_rt::entry;
use microbit::{
    board::Board,
    display::blocking::Display,
    hal::{
        gpio::{Level, OpenDrainConfig}, saadc::{Channel, SaadcConfig}, timer::{self, OneShot, Periodic}, Saadc, Timer
    }, pac::{saadc::SAMPLERATE, CLOCK, TIMER0},
};

const TIMER_TICKS_PER_SECOND: u32 = 1_000_000;

const INIT_TEST_SAMPLES: u32 = 10_000;

#[entry]
fn main() -> ! {
    rtt_init_print!();

    let mut board = Board::take().unwrap();
    let timer = Timer::new(board.TIMER0);
    let mut timer = timer.into_periodic();
    
    let mut display = Display::new(board.display_pins);

    // initialize adc
    let saadc_config = SaadcConfig::default();
    let mut saadc = Saadc::new(board.ADC, saadc_config);
    let mut mic_in = board.microphone_pins.mic_in.into_floating_input();

    board
        .microphone_pins
        .mic_run
        .into_open_drain_output(OpenDrainConfig::Disconnect0HighDrive1, Level::High);

    let middle = find_middle(&mut saadc, &mut mic_in, INIT_TEST_SAMPLES);
    
    let mut samples = 0;
    let mut last_sample = 0;
    let mut sample = 0;
    let mut middle_crosses = 0;

    timer.start(TIMER_TICKS_PER_SECOND);

    loop {
        sample = saadc.read_channel(&mut mic_in).expect("could not read value of microphone") as u16;
        samples += 1;
        if is_between(middle, last_sample, sample) {
            middle_crosses += 1;
        }
        last_sample = sample;
        if timer.reset_if_finished() {
            rprintln!("{}", middle_crosses / 2);
            samples = 0;
            middle_crosses = 0;
        }
        // rprintln!("{}, avg={}, max={}, min={}", mic_value, sum / count, max, min);
    }
}

fn find_middle<PIN>(saadc: &mut Saadc, mic_in: &mut PIN, samples: u32) -> u16 
where
    PIN: Channel
{
    let mut total: u32 = 0;
    for _ in 0..samples {
        let mic_value = saadc
            .read_channel(mic_in)
            .expect("could not read value of microphone") as u16;
        total += mic_value as u32;
    }
    return (total / samples) as u16;
}

fn is_between(mid: u16, a: u16, b: u16) -> bool {
    (a <= mid && b > mid) || (a >= mid && b < mid)
}

// Is there a way to make this generic over the actual clock?
fn timer_wait(timer: &mut Timer<TIMER0, Periodic>) {
    loop {
        match timer.reset_if_finished() {
            true => {
                return;
            },
            false => {},
        }
    }
}