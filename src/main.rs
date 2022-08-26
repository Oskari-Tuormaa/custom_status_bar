#[macro_use]
mod modules;

use std::{
    thread::sleep,
    time::Duration,
};

use modules::*;

fn main() {
    let mut modules: Vec<Box<dyn Module>> = boxvec![
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
