#[macro_use]
mod modules;

use std::{thread::sleep, time::Duration};

use modules::*;

fn main() {
    let mut modules = modules![
        BatteryModule::new([
            "/sys/class/power_supply/BAT0",
        ]),
        NetworkModule::new("wlp0s20f3").with_name("W"),
        DiskSpaceModule::new("/dev/mapper/vgubuntu-root"),
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
