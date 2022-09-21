#[macro_use]
mod modules;

use std::{thread::sleep, time::Duration};

use modules::*;

fn main() {
    let mut modules = modules![
        NetworkModule::new("enp4s0").with_name("E"),
        DiskSpaceModule::new("/dev/sda3"),
        DiskSpaceModule::new("/dev/sdb1"),
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
