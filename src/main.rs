#![no_std]
#![no_main]

// use defmt_rtt as _;
use rtt_target::{rtt_init_print, rprintln};
use panic_rtt_target as _;
// use panic_halt as _;

use cortex_m_rt::entry;
use microbit::{
    board::Board,
    display::blocking::Display,
    hal::{
        gpio::{Level, OpenDrainConfig}, saadc::SaadcConfig, timer::{OneShot, Periodic}, Saadc, Timer
    }, pac::{saadc::SAMPLERATE, TIMER0},
};

const SAMPLE_RATE: u64 = 8_000;
const SAMPLE_DELAY: u64 = 1_000_000 / SAMPLE_RATE;
const SAMPLE_PERIOD_US: u32 = 125;

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

    let mut count = 0;
    let mut sum = 0;
    let (mut max, mut min) = (0, 0);

    timer.start(SAMPLE_PERIOD_US);

    loop {
        timer_wait(&mut timer);
        // rprintln!("cycles={}", cycles);
        let mic_value = saadc.read_channel(&mut mic_in).expect("could not read value of microphone") as u16;
        count += 1;
        if count % 10000 == 0 {
            max = sum / count;
            min = sum / count;
        }
        sum += mic_value as u64;
        max = max.max(mic_value as u64);
        min = min.min(mic_value as u64);
        // rprintln!("{}, avg={}, max={}, min={}", mic_value, sum / count, max, min);
        
    }
}

fn timer_wait(timer: &mut Timer<TIMER0, Periodic>) {
    loop {
        match timer.reset_if_finished() {
            true => {
                return;
            },
            false => {},
        }
        // rprintln!("stuck :(");
    }
}