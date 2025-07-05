#![no_std]
#![no_main]

use core::{i32, ops::RangeInclusive};

// use defmt_rtt as _;
use rtt_target::{rprint, rprintln, rtt_init_print};
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
const SAMPLES: usize = 2048;
const LAG_MIN: usize = 8;
const LAG_MAX: usize = 400;
const GAIN_LIMIT: f32 = 1000.;

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
    
    let mut samples = [0.; SAMPLES];
    let mut sample_index = 0;

    timer.start(SAMPLE_PERIOD_CYCLES as u32);

    loop {
        timer_wait(&mut timer);
        let sample = saadc.read_channel(&mut mic_in).expect("could not read value of microphone") as f32;
        samples[sample_index] = sample;
        sample_index += 1;
        if sample_index == SAMPLES {
            // Subtract the mean
            zero_shift(&mut samples);
            // Amplify
            amplify(&mut samples, GAIN_LIMIT);
            let frequency = asmd_frequency(&samples[..], LAG_MIN..=LAG_MAX, SAMPLE_RATE);
            rprintln!("{}", frequency);
            // Clean up
            sample_index = 0;
        }
    }
}

// shift samples down to around 0.0. Samples are assumed to all be positive
fn zero_shift(data: &mut [f32]) {
    let mean = data.iter().sum::<f32>() / data.len() as f32;
    for sample in data.iter_mut() {
        *sample -= mean;
    }
}

// Amplify data. 
// The microphone data has quite low amplitude. 
// This may limit the effectiveness of autocorrelation
fn amplify(data: &mut [f32], gain_limit: f32) {
    let abs_value_max = data.iter().map(|s| s.abs()).reduce(f32::max).unwrap();
    let gain = GAIN_LIMIT / abs_value_max;
    for sample in data.iter_mut() {
        *sample *= gain;
    }
}

fn asmd_frequency(samples: &[f32], lag_range: RangeInclusive<usize>, sample_rate: usize) -> f32 {
    let mut min_asmd = f32::MAX;
    let mut closest_matching_lag = 0;
    for lag in lag_range {
        let asmd = asmd(samples, lag);
        if asmd < min_asmd && asmd != 0. {
            min_asmd = asmd;
            closest_matching_lag = lag;
        }
        // rprint!("{} ", asmd);
    }
    sample_rate as f32 / closest_matching_lag as f32
}

// Averaged square mean difference
fn asmd(x: &[f32], lag: usize) -> f32 {
    let n = x.len();
    let mut total = 0.;
    for i in 0..=n - lag - 1 {
        let diff = x[i] - x[i + lag];
        total += diff * diff;
    }
    total / (n - lag) as f32
}

// Is there a way to make this generic over the actual clock?
fn timer_wait(timer: &mut Timer<TIMER0, Periodic>) {
    while !timer.reset_if_finished() {}
}