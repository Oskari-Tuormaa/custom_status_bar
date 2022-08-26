use chrono::Local;
use std::{thread::sleep, time::Duration};
use sysinfo::{ComponentExt, CpuExt, System, SystemExt};

macro_rules! boxvec {
    ($($x:expr),*) => {
        vec![ $(Box::new($x)),* ]
    };
}

#[derive(Default)]
pub struct ModuleOutput {
    content: String,
    color_fg: Option<String>,
    color_bg: Option<String>,
    border: Option<String>,
}

impl ModuleOutput {
    pub fn new(content: String) -> Self {
        ModuleOutput {
            content,
            color_fg: None,
            color_bg: None,
            border: None,
        }
    }

    pub fn with_color_fg(mut self, color: String) -> Self {
        self.color_fg = Some(color);
        self
    }

    pub fn with_color_bg(mut self, color: String) -> Self {
        self.color_bg = Some(color);
        self
    }

    pub fn with_border(mut self, border: String) -> Self {
        self.border = Some(border);
        self
    }
}

pub trait Module {
    fn get_output(&mut self) -> ModuleOutput;
}

pub fn combine_modules(modules: &mut Vec<Box<dyn Module>>) -> String {
    let mut res = String::from("[");

    if let Some(mods) = modules
        .iter_mut()
        .map(|v| {
            let modout = v.get_output();
            let mut res_inner = String::with_capacity(20);
            res_inner += &format!("{{\"full_text\": \"{}\"", modout.content);
            let map_optional = |key, val: Option<String>| {
                val.map(|v| format!(", \"{}\": \"{}\"", key, v))
                    .unwrap_or("".to_string())
            };
            res_inner += &map_optional("color", modout.color_fg);
            res_inner += &map_optional("background", modout.color_bg);
            res_inner += &map_optional("border", modout.border);
            res_inner += "}";
            res_inner
        })
        .reduce(|a, n| a + ", " + &n)
    {
        res += &mods;
    }

    res += "]";
    res
}

pub struct DateTimeModule;

impl Module for DateTimeModule {
    fn get_output(&mut self) -> ModuleOutput {
        let now = Local::now();
        ModuleOutput::new(now.format("%d/%m/%y %H:%M:%S").to_string())
    }
}

pub struct RamModule {
    system: System,
}

impl RamModule {
    pub fn new() -> Self {
        RamModule {
            system: System::new(),
        }
    }
}

impl Module for RamModule {
    fn get_output(&mut self) -> ModuleOutput {
        self.system.refresh_memory();

        let ktog = |v| v as f32 / 1024. / 1024.;
        ModuleOutput::new(format!(
            "{:.1}/{:.1} GiB",
            ktog(self.system.used_memory()),
            ktog(self.system.total_memory())
        ))
    }
}

fn percentage_to_char(v: f32) -> Option<char> {
    let v = (7. * v / 100.) as u32;
    char::from_u32(0x2581 + v)
}

pub struct CpuModule {
    system: System,
}

impl CpuModule {
    pub fn new() -> Self {
        CpuModule {
            system: System::new(),
        }
    }
}

impl Module for CpuModule {
    fn get_output(&mut self) -> ModuleOutput {
        self.system.refresh_cpu();
        sleep(Duration::from_millis(200));
        self.system.refresh_cpu();

        let cpus = self.system.cpus();

        let mut out = ModuleOutput::new(
            cpus.iter()
                .map(|c| percentage_to_char(c.cpu_usage()).unwrap_or(' '))
                .fold("".to_string(), |a, n| a + &n.to_string()),
        )
        .with_color_bg("#44475a".to_string())
        .with_border("#000000".to_string());

        let load = cpus.iter().map(|v| v.cpu_usage()).sum::<f32>() / cpus.len() as f32;
        if load > 80. {
            out = out.with_color_fg("#ff5555".to_string());
        }

        out
    }
}

pub struct TemperatureModule {
    system: System,
}

impl TemperatureModule {
    pub fn new() -> Self {
        TemperatureModule {
            system: System::new(),
        }
    }
}

impl Module for TemperatureModule {
    fn get_output(&mut self) -> ModuleOutput {
        self.system.refresh_components_list();
        self.system.refresh_components();

        ModuleOutput::new(
            if let Some(cpu) = self.system.components().iter().find(|c| c.label() == "CPU") {
                format!("{}°C", cpu.temperature())
            } else {
                "Temp unavailable".to_string()
            },
        )
    }
}
