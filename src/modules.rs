use chrono::Local;
use dbus::blocking::Connection;
use networkmanager::{
    devices::{Any, Device, Wired, Wireless},
    NetworkManager,
};
use std::fmt::{Display, Write};
use std::{fs::read_to_string, net::Ipv4Addr, path::PathBuf, thread::sleep, time::Duration};
use sysinfo::{ComponentExt, CpuExt, DiskExt, System, SystemExt};

#[derive(Default)]
pub struct ModuleOutput {
    content: String,
    color_fg: Option<String>,
    color_bg: Option<String>,
    border: Option<String>,
    separator: Option<bool>,
    separator_block_width: Option<usize>,
}

impl ModuleOutput {
    pub fn new(content: String) -> Self {
        ModuleOutput {
            content,
            color_fg: None,
            color_bg: None,
            border: None,
            separator: None,
            separator_block_width: None,
        }
    }

    pub fn with_content(mut self, content: String) -> Self {
        self.content = content;
        self
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

    pub fn with_separator(mut self, seperator: bool) -> Self {
        self.separator = Some(seperator);
        self
    }

    pub fn with_separator_block_width(mut self, separator_block_width: usize) -> Self {
        self.separator_block_width = Some(separator_block_width);
        self
    }
}

type ModuleRes = Result<ModuleOutput, Option<String>>;
pub trait Module {
    fn get_output(&mut self) -> ModuleRes;
    fn rate(&self) -> usize {
        1
    }
}

macro_rules! modules {
    ($($x:expr),*) => {
        Modules::new([ $(Box::new($x)),* ])
    };
}

pub struct Modules<const N: usize> {
    modules: [Box<dyn Module>; N],
    cache: [Option<String>; N],
    tick: usize,
}

fn map_optional(key: &str, val: Option<impl Display>) -> String {
    val.map(|v| format!(", \"{}\": {}", key, v))
        .unwrap_or_else(|| "".to_string())
}

fn map_optional_quotes(key: &str, val: Option<impl Display>) -> String {
    val.map(|v| format!(", \"{}\": \"{}\"", key, v))
        .unwrap_or_else(|| "".to_string())
}

impl<const N: usize> Modules<N> {
    pub fn new(modules: [Box<dyn Module>; N]) -> Self {
        Modules {
            modules,
            cache: [(); N].map(|_| None),
            tick: 0,
        }
    }

    pub fn combine_modules(&mut self) -> String {
        let mut res = String::from("[");

        if let Some(mods) = self
            .modules
            .iter_mut()
            .enumerate()
            .filter_map(|(i, v)| {
                if self.tick % v.rate() != 0 {
                    return self.cache[i].clone();
                }

                let mut res_inner = String::with_capacity(20);
                match v.get_output() {
                    Ok(modout) => {
                        write!(res_inner, "{{\"full_text\": \"{}\"", modout.content).unwrap();
                        res_inner += &map_optional_quotes("color", modout.color_fg);
                        res_inner += &map_optional_quotes("background", modout.color_bg);
                        res_inner += &map_optional_quotes("border", modout.border);
                        res_inner += &map_optional("separator", modout.separator);
                        res_inner +=
                            &map_optional("separator_block_width", modout.separator_block_width);
                        res_inner += "}";
                    }
                    Err(Some(mes)) if !mes.is_empty() => {
                        write!(
                            res_inner,
                            "{{\"full_text\": \"{}\", \"color\": \"#ff0000\"}}",
                            mes
                        )
                        .unwrap();
                    }
                    Err(_) => {
                        if v.rate() > 1 {
                            self.cache[i] = None;
                        }
                        return None;
                    }
                }

                if v.rate() > 1 {
                    self.cache[i] = Some(res_inner.clone());
                }
                Some(res_inner)
            })
            .reduce(|a, n| a + ", " + &n)
        {
            res += &mods;
        }

        res += "]";
        self.tick += 1;
        res
    }
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

    fn rate(&self) -> usize {
        3
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
            .ok_or_else(|| "CPU unavailable".to_string())?;

        Ok(ModuleOutput::new(format!("{}°C", cpu.temperature())))
    }

    fn rate(&self) -> usize {
        5
    }
}

pub struct DiskSpaceModule {
    dev: &'static str,
    system: System,
}

impl DiskSpaceModule {
    pub fn new(dev: &'static str) -> Self {
        DiskSpaceModule {
            dev,
            system: System::new(),
        }
    }
}

impl Module for DiskSpaceModule {
    fn get_output(&mut self) -> ModuleRes {
        self.system.refresh_disks();
        self.system.refresh_disks_list();

        let disk = self
            .system
            .disks()
            .iter()
            .find(|d| d.name() == self.dev)
            .ok_or_else(|| "Disk unavailable".to_string())?;

        Ok(ModuleOutput::new(format!(
            "{} GiB",
            disk.available_space() / 1024u64.pow(3)
        )))
    }

    fn rate(&self) -> usize {
        5
    }
}

pub struct NetworkModule {
    device: &'static str,
    name: Option<&'static str>,
}

impl NetworkModule {
    pub fn new(device: &'static str) -> Self {
        NetworkModule { device, name: None }
    }

    pub fn with_name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }
}

impl Module for NetworkModule {
    fn get_output(&mut self) -> ModuleRes {
        let dbus = Connection::new_system().map_err(|_| "dbus unavailable".to_string())?;
        let nm = NetworkManager::new(&dbus);

        let name = self.name.unwrap_or(self.device);
        let dev = nm.get_device_by_ip_iface(self.device).map_err(|_| None)?;

        let ip_from_addr = |addr: Vec<Vec<u32>>| {
            addr.iter()
                .flatten()
                .next()
                .map(|ip| format!(" {}", Ipv4Addr::from(ip.to_be())))
                .unwrap_or_else(|| "".to_string())
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
            _ => Err(Some("Unsupported device".to_string())),
        }
    }

    fn rate(&self) -> usize {
        5
    }
}

pub struct BatteryModule<const N: usize> {
    dev_path: [PathBuf; N],
}

impl<const N: usize> BatteryModule<N> {
    pub fn new(path: [&str; N]) -> Self {
        BatteryModule {
            dev_path: path.map(PathBuf::from),
        }
    }
}

impl<const N: usize> Module for BatteryModule<N> {
    fn get_output(&mut self) -> ModuleRes {
        let get_measure = |file: &str| {
            self.dev_path
                .iter()
                .map(|p| {
                    read_to_string(p.join(file))
                        .map(|v| v.trim().parse::<u64>().ok())
                        .ok()
                        .flatten()
                })
                .reduce(|a, n| Some(a? + n?))
                .flatten()
        };
        let ecap = get_measure("charge_full").ok_or(None)?;
        let enow = get_measure("charge_now").ok_or(None)?;
        let cnow = get_measure("current_now").ok_or(None)?;
        let perc = (100 * enow) / ecap;

        let mut hours_left = 0.;
        let mut mins_left = 0.;
        let mut secs_left = 0.;

        let mut out = ModuleOutput::new("".to_string());
        let bat = char::from_u32(0xf244 - ((4 * perc) / 100) as u32).unwrap_or('');
        if let Some(state) = self
            .dev_path
            .iter()
            .map(|p| match read_to_string(p.join("status")) {
                Ok(mes) if mes.trim() == "Charging" => 1,
                Ok(mes) if mes.trim() == "Discharging" => -1,
                _ => 0,
            })
            .find(|v| *v != 0)
        {
            match state {
                1 => {
                    out = out.with_color_fg("#50fa7b".to_string());
                    hours_left = (ecap - enow) as f32 / cnow as f32;
                }
                -1 => {
                    out = out.with_color_fg("#ff5555".to_string());
                    hours_left = enow as f32 / cnow as f32;
                }
                _ => (),
            }
        }
        mins_left = hours_left.fract() * 60.;

        if hours_left.floor() > 0.0 {
            out = out.with_content(format!(
                "{} {}% [{:.0}h {:.0}m]",
                bat,
                perc,
                hours_left.floor(),
                mins_left.floor()
            ));
        } else if mins_left > 0.0 {
            out = out.with_content(format!(
                "{} {}% [{:.0}m]",
                bat,
                perc,
                mins_left.floor()
            ));
        } else {
            out = out.with_content(format!(
                "{} {}%",
                bat,
                perc,
            ));

        }

        Ok(out)
    }

    fn rate(&self) -> usize {
        5
    }
}

pub struct SpacerModule<const N: usize> {
    data: String,
}

impl<const N: usize> SpacerModule<N> {
    pub fn new() -> Self {
        let mut data = String::with_capacity(N);
        for _ in 0..N {
            data.push(' ');
        }
        SpacerModule { data }
    }
}

impl<const N: usize> Module for SpacerModule<N> {
    fn get_output(&mut self) -> ModuleRes {
        Ok(ModuleOutput::new(self.data.clone()))
    }
}
