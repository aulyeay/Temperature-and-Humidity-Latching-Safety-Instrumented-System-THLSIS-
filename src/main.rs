use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::*;
use esp_idf_svc::hal::peripherals::Peripherals;

const THRESHOLD_WARNING: f32 = 26.0;
const THRESHOLD_TRIP: f32 = 30.0;
const VOTER_REQUIRED: u8 = 3;

fn simulate_temperature(index: u32) -> f32 {
    match index {
        1..=15  => 22.0 + (index as f32) * 0.13,
        16..=30 => 26.5 + ((index - 15) as f32) * 0.15,
        _       => 30.5 + ((index - 30) as f32) * 0.05,
    }
}

fn main() {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let mut led_pin   = PinDriver::output(peripherals.pins.gpio2).unwrap();
    let mut relay_pin = PinDriver::output(peripherals.pins.gpio10).unwrap();
    let btn_pin = PinDriver::input(peripherals.pins.gpio0, Pull::Up).unwrap();

    relay_pin.set_high().unwrap();
    led_pin.set_low().unwrap();

    let mut latch_alarm: bool = false;
    let mut latch_sis:   bool = false;
    let mut counter_warning: u8 = 0;
    let mut counter_trip:    u8 = 0;
    let mut sample_index:   u32 = 0;

    log::info!("=== SAFE-Rust X-Ray Room Monitor ===");
    log::info!("Warning : {:.1} C | Trip : {:.1} C", THRESHOLD_WARNING, THRESHOLD_TRIP);

    loop {
        sample_index += 1;

        // 1. CEK RESET BUTTON
        if btn_pin.is_low() {
            latch_alarm     = false;
            latch_sis       = false;
            counter_warning = 0;
            counter_trip    = 0;
            relay_pin.set_high().unwrap();
            led_pin.set_low().unwrap();
            log::info!("RESET: SR Latch cleared");
            FreeRtos::delay_ms(300);
            continue;
        }

        // 2. BACA SENSOR (simulasi dulu, nanti ganti DHT22)
        let temp = simulate_temperature(sample_index);
        let hum: f32 = 60.0;

        // 3. VOTING LOGIC (N-of-3)
        if temp >= THRESHOLD_TRIP {
            if counter_trip    < VOTER_REQUIRED { counter_trip    += 1; }
            if counter_warning < VOTER_REQUIRED { counter_warning += 1; }
            if counter_trip    >= VOTER_REQUIRED { latch_sis   = true; }
            if counter_warning >= VOTER_REQUIRED { latch_alarm = true; }
        } else if temp >= THRESHOLD_WARNING {
            counter_trip = 0;
            if counter_warning < VOTER_REQUIRED { counter_warning += 1; }
            if counter_warning >= VOTER_REQUIRED { latch_alarm = true; }
        } else {
            counter_trip    = 0;
            counter_warning = 0;
        }

        // 4. UART OUTPUT format CSV → untuk Excel & GNUPlot
        log::info!("{},{:.2},{:.1},{},{}",
            sample_index, temp, hum,
            latch_alarm as u8,
            latch_sis   as u8
        );

        // 5. AKTUATOR
        if latch_sis {
            relay_pin.set_low().unwrap();
            led_pin.set_high().unwrap();
            log::info!("[{}] {:.2}C | STATE: TRIP SIS", sample_index, temp);
        } else if latch_alarm {
            relay_pin.set_high().unwrap();
            led_pin.set_high().unwrap();
            log::info!("[{}] {:.2}C | STATE: WARNING", sample_index, temp);
        } else {
            relay_pin.set_high().unwrap();
            led_pin.set_low().unwrap();
            log::info!("[{}] {:.2}C | STATE: NORMAL", sample_index, temp);
        }

        FreeRtos::delay_ms(2000);
    }
}