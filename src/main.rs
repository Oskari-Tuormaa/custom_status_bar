#[macro_use]
mod modules;

use std::{thread::sleep, time::Duration};

use modules::*;

fn main() {
    let mut modules = modules![
        BatteryModule::new([
            "/sys/class/power_supply/BAT0",
        ]),
        NetworkModule::new("enp0s13f0u1u1").with_name("E"),
        NetworkModule::new("enp0s13f0u2u1").with_name("E"),
        NetworkModule::new("wlan0").with_name("W"),
        DiskSpaceModule::new("/dev/nvme0n1p3"),
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
