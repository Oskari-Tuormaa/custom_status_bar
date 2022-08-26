#[macro_use]
mod modules;

use std::{thread::sleep, time::Duration};

use modules::*;

fn main() {
    let mut modules: Vec<Box<dyn Module>> = boxvec![
        NetworkModule::new("enp0s31f6").with_name("E"),
        NetworkModule::new("enp60s0u1u1").with_name("ED"),
        NetworkModule::new("wlp3s0").with_name("W"),
        DiskSpaceModule::new("/dev/sda3"),
        TemperatureModule::new(),
        RamModule::new(),
        CpuModule::new(),
        DateTimeModule
    ];
    println!("{{\"version\": 1}}\n[");

    loop {
        let res = combine_modules(&mut modules);
        println!("{},", res);
        sleep(Duration::from_secs_f32(0.5));
    }
}
