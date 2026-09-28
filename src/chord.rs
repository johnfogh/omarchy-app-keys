use std::fs::File;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::thread;
use std::time::Duration;

const EV_KEY: i32 = 0x01;
const EV_SYN: i32 = 0x00;
const UI_DEV_CREATE: u64 = 0x5501;
const UI_DEV_DESTROY: u64 = 0x5502;

const KEYS: &[(&str, i32)] = &[
    ("a", 30), ("b", 48), ("c", 46), ("d", 32), ("e", 18), ("f", 33), ("g", 34), ("h", 35),
    ("i", 23), ("j", 36), ("k", 37), ("l", 38), ("m", 50), ("n", 49), ("o", 24), ("p", 25),
    ("q", 16), ("r", 19), ("s", 31), ("t", 20), ("u", 22), ("v", 47), ("w", 17), ("x", 45),
    ("y", 21), ("z", 44),
    ("1", 2), ("2", 3), ("3", 4), ("4", 5), ("5", 6), ("6", 7), ("7", 8), ("8", 9), ("9", 10), ("0", 11),
    ("minus", 12), ("-", 12), ("equal", 13), ("=", 13), ("tab", 15), ("space", 57),
    ("super", 125), ("super_l", 125),
];

const MODS: &[(&str, i32)] = &[("super", 125), ("ctrl", 29), ("shift", 42), ("alt", 56)];

fn iow(nr: u64, size: u64) -> u64 {
    (1 << 30) | (size << 16) | ((b'U' as u64) << 8) | nr
}

fn lookup(table: &[(&str, i32)], name: &str) -> Result<i32, String> {
    table
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, code)| *code)
        .ok_or_else(|| format!("unknown key {name}"))
}

fn ioctl(fd: i32, request: u64, arg: libc::c_ulong) -> std::io::Result<()> {
    let rc = unsafe { libc::ioctl(fd, request, arg) };
    if rc < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn emit(file: &mut File, type_: u16, code: u16, value: i32) -> std::io::Result<()> {
    let mut event = [0u8; 24];
    event[16..18].copy_from_slice(&type_.to_ne_bytes());
    event[18..20].copy_from_slice(&code.to_ne_bytes());
    event[20..24].copy_from_slice(&value.to_ne_bytes());
    file.write_all(&event)
}

/// Press `mod+mod+key` through uinput so Hyprland handles it as a real chord.
pub fn press_chord(chord: &str) -> Result<(), String> {
    let parts: Vec<&str> = chord.split('+').filter(|part| !part.is_empty()).collect();
    let (key_name, mod_names) = parts.split_last().ok_or("empty chord")?;
    let key = lookup(KEYS, key_name)?;
    let mods = mod_names
        .iter()
        .map(|name| lookup(MODS, name))
        .collect::<Result<Vec<_>, _>>()?;

    let file = File::options()
        .write(true)
        .open("/dev/uinput")
        .map_err(|err| err.to_string())?;
    let fd = file.as_raw_fd();
    ioctl(fd, iow(100, 4), EV_KEY as libc::c_ulong).map_err(|err| err.to_string())?;
    for code in mods.iter().copied().chain(std::iter::once(key)) {
        ioctl(fd, iow(101, 4), code as libc::c_ulong).map_err(|err| err.to_string())?;
    }
    let mut setup = [0u8; 8 + 80 + 4];
    setup[0..2].copy_from_slice(&0x03u16.to_ne_bytes());
    setup[8..8 + b"app-layer-chord".len()].copy_from_slice(b"app-layer-chord");
    ioctl(fd, iow(3, setup.len() as u64), setup.as_ptr() as libc::c_ulong).map_err(|err| err.to_string())?;
    ioctl(fd, UI_DEV_CREATE, 0).map_err(|err| err.to_string())?;
    thread::sleep(Duration::from_millis(50));

    let mut file = file;
    let send = |file: &mut File, code: i32, value: i32| -> std::io::Result<()> {
        emit(file, EV_KEY as u16, code as u16, value)?;
        emit(file, EV_SYN as u16, 0, 0)
    };
    let result = (|| {
        for code in &mods {
            send(&mut file, *code, 1)?;
        }
        send(&mut file, key, 1)?;
        send(&mut file, key, 0)?;
        for code in mods.iter().rev() {
            send(&mut file, *code, 0)?;
        }
        thread::sleep(Duration::from_millis(20));
        Ok(())
    })();
    let _ = ioctl(fd, UI_DEV_DESTROY, 0);
    result.map_err(|err: std::io::Error| err.to_string())
}
