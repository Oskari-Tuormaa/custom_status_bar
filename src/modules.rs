use chrono::Local;
use dbus::blocking::Connection;
use networkmanager::{
    devices::{Any, Device, Wired, Wireless},
    NetworkManager,
};
use std::{net::Ipv4Addr, thread::sleep, time::Duration};
use sysinfo::{ComponentExt, CpuExt, DiskExt, System, SystemExt};

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

type ModuleRes = Result<ModuleOutput, String>;
pub trait Module {
    fn get_output(&mut self) -> ModuleRes;
}

pub fn combine_modules(modules: &mut Vec<Box<dyn Module>>) -> String {
    let mut res = String::from("[");

    if let Some(mods) = modules
        .iter_mut()
        .filter_map(|v| {
            let mut res_inner = String::with_capacity(20);
            match v.get_output() {
                Ok(modout) => {
                    res_inner += &format!("{{\"full_text\": \"{}\"", modout.content);
                    let map_optional = |key, val: Option<String>| {
                        val.map(|v| format!(", \"{}\": \"{}\"", key, v))
                            .unwrap_or("".to_string())
                    };
                    res_inner += &map_optional("color", modout.color_fg);
                    res_inner += &map_optional("background", modout.color_bg);
                    res_inner += &map_optional("border", modout.border);
                    res_inner += "}";
                }
                Err(mes) if !mes.is_empty() => {
                    res_inner += &format!("{{\"full_text\": \"{}\", \"color\": \"#ff0000\"}}", mes);
                }
                Err(_) => return None,
            }
            Some(res_inner)
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
    fn get_output(&mut self) -> ModuleRes {
        let now = Local::now();
        Ok(ModuleOutput::new(now.format("%d/%m/%y %H:%M").to_string()))
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
    fn get_output(&mut self) -> ModuleRes {
        self.system.refresh_memory();

        let ktog = |v| v as f32 / 1024. / 1024.;
        Ok(ModuleOutput::new(format!(
            "{:.1}/{:.1} GiB",
            ktog(self.system.used_memory()),
            ktog(self.system.total_memory())
        )))
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
    fn get_output(&mut self) -> ModuleRes {
        self.system.refresh_cpu();
        sleep(Duration::from_millis(200));
        self.system.refresh_cpu();

        let cpus = self.system.cpus();

        let cpu_sparkline = cpus
            .iter()
            .map(|c| percentage_to_char(c.cpu_usage()).unwrap_or(' '))
            .fold("".to_string(), |a, n| a + &n.to_string());

        let mut out = ModuleOutput::new(cpu_sparkline)
            .with_color_bg("#44475a".to_string())
            .with_border("#000000".to_string());

        if self.system.global_cpu_info().cpu_usage() > 80. {
            out = out.with_color_fg("#ff5555".to_string());
        }

        Ok(out)
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
    fn get_output(&mut self) -> ModuleRes {
        self.system.refresh_components_list();
        self.system.refresh_components();

        let cpu = self
            .system
            .components()
            .iter()
            .find(|c| c.label() == "CPU")
            .ok_or("CPU unavailable".to_string())?;

        Ok(ModuleOutput::new(format!("{}°C", cpu.temperature())))
    }
}

pub struct DiskSpaceModule<'a> {
    dev: &'a str,
    system: System,
}

impl<'a> DiskSpaceModule<'a> {
    pub fn new(dev: &'a str) -> Self {
        DiskSpaceModule {
            dev,
            system: System::new(),
        }
    }
}

impl<'a> Module for DiskSpaceModule<'a> {
    fn get_output(&mut self) -> ModuleRes {
        self.system.refresh_disks();
        self.system.refresh_disks_list();

        let disk = self
            .system
            .disks()
            .iter()
            .find(|d| d.name() == &self.dev[..])
            .ok_or("Disk unavailable".to_string())?;

        Ok(ModuleOutput::new(format!(
            "{} GiB",
            disk.available_space() / 1024u64.pow(3)
        )))
    }
}

pub struct NetworkModule<'a> {
    device: &'a str,
    name: Option<&'a str>,
}

impl<'a> NetworkModule<'a> {
    pub fn new(device: &'a str) -> Self {
        NetworkModule { device, name: None }
    }

    pub fn with_name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }
}

impl<'a> Module for NetworkModule<'a> {
    fn get_output(&mut self) -> ModuleRes {
        let dbus = Connection::new_system().map_err(|_| "dbus unavailable".to_string())?;
        let nm = NetworkManager::new(&dbus);

        let name = self.name.unwrap_or(self.device);
        let dev = nm.get_device_by_ip_iface(self.device).map_err(|_| "")?;

        let ip_from_addr = |addr: Vec<Vec<u32>>| {
            addr.iter()
                .flatten()
                .next()
                .map(|ip| format!(" {}", Ipv4Addr::from(ip.to_be())))
                .unwrap_or("".to_string())
        };
        match dev {
            Device::WiFi(dev) => {
                let ap = dev.active_access_point().unwrap();
                if let (Ok(ssid), Ok(strength), Ok(freq), Ok(Ok(addr))) = (
                    ap.ssid(),
                    ap.strength(),
                    ap.frequency(),
                    dev.ip4_config().map(|conf| conf.addresses()),
                ) {
                    Ok(ModuleOutput::new(format!(
                        "{}: ({:3}% at {}, {} Mb/s){}",
                        name,
                        strength,
                        ssid,
                        freq / 1024,
                        ip_from_addr(addr)
                    ))
                    .with_color_fg("#50fa7b".to_string()))
                } else {
                    Ok(ModuleOutput::new(format!("{}: down", name))
                        .with_color_fg("#ff5555".to_string()))
                }
            }
            Device::Ethernet(dev) => {
                if let (Ok(true), Ok(speed), Ok(Ok(addr))) = (
                    dev.carrier(),
                    dev.speed(),
                    dev.ip4_config().map(|conf| conf.addresses()),
                ) {
                    Ok(ModuleOutput::new(format!(
                        "{}: ({} Mb/s){}",
                        name,
                        speed,
                        ip_from_addr(addr)
                    ))
                    .with_color_fg("#50fa7b".to_string()))
                } else {
                    Ok(ModuleOutput::new(format!("{}: down", name))
                        .with_color_fg("#ff5555".to_string()))
                }
            }
            _ => return Err("Unsupported device".to_string()),
        }
    }
}
