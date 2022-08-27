#[macro_use]
mod modules;

use std::{thread::sleep, time::Duration};

use modules::*;

fn main() {
    let mut modules = modules![
        BatteryModule::new([
            "/sys/class/power_supply/BAT0",
            "/sys/class/power_supply/BAT1"
        ]),
        NetworkModule::new("enp0s31f6").with_name("E"),
        NetworkModule::new("enp60s0u1u1").with_name("ED"),
        NetworkModule::new("wlp3s0").with_name("W"),
        DiskSpaceModule::new("/dev/sda3"),
        TemperatureModule::new(),
        RamModule::new(),
        CpuModule::new(),
        DateTimeModule,
        SpacerModule::<0>::new()
    ];
    println!("{{\"version\": 1}}\n[");

    let t_sleep = Duration::from_millis(1000);
    loop {
        let res = modules.combine_modules();
        println!("{},", res);
        sleep(t_sleep);
    }
}
