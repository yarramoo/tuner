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

const TIMER_TICKS_PER_SECOND: usize = 1_000_000;
const SAMPLE_RATE: usize = 8_000;
const SAMPLE_PERIOD_CYCLES: usize = TIMER_TICKS_PER_SECOND / SAMPLE_RATE;
const SAMPLES: usize = 500;
const LAG_MIN: usize = 8;
const LAG_MAX: usize = 100;

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

    let sam
    
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

fn asmd_frequency(samples: &[u16], lag_range: RangeInclusive<usize>, sample_rate: usize) -> usize {
    let mut min_asmd = usize::MAX;
    let mut closest_matching_lag = 0;
    for lag in lag_range {
        let asmd = asmd(samples, lag);
        if asmd < min_asmd {
            min_asmd = asmd;
            closest_matching_lag = lag;
        }
    }
    sample_rate / closest_matching_lag
}

// Averaged square mean difference
fn asmd(x: &[u16], lag: usize) -> usize {
    let n = x.len();
    let mut total = 0;
    for i in 0..n - lag - 1 {
        total += (x[i] - x[i + lag]).pow(2) as usize;
    }
    total / (n - lag)
}