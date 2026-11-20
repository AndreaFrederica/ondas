use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    process::Command,
};

pub(crate) fn reader_dbg_version(path: &Path) -> Option<u16> {
    let sections = Command::new("readelf")
        .args(["-SW"])
        .arg(path)
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    if !sections.status.success() {
        return None;
    }
    let sections = String::from_utf8_lossy(&sections.stdout);
    let (address, offset, size) = sections.lines().find_map(|line| {
        let fields: Vec<_> = line.split_whitespace().collect();
        let index = fields.iter().position(|field| *field == ".rodata")?;
        let hex = |at: usize| u64::from_str_radix(fields.get(at)?, 16).ok();
        Some((hex(index + 2)?, hex(index + 3)?, hex(index + 4)?))
    })?;
    let code = Command::new("objdump")
        .args([
            "-d",
            "--disassemble=_ZN9SimBridge12DbgReaderImp13readDbgHeaderEv",
        ])
        .arg(path)
        .env("LC_ALL", "C")
        .output()
        .ok()?;
    if !code.status.success() {
        return None;
    }
    let mut file = File::open(path).ok()?;
    let mut versions = BTreeSet::new();
    // Read only constants referenced by the SDK's header reader. Other DBG
    // strings can describe backwards compatibility instead of its own version.
    for line in String::from_utf8_lossy(&code.stdout).lines() {
        let Some(reference) = line
            .split_once('#')
            .and_then(|(_, text)| u64::from_str_radix(text.split_whitespace().next()?, 16).ok())
        else {
            continue;
        };
        let Some(relative) = reference.checked_sub(address) else {
            continue;
        };
        if relative.checked_add(20)? > size {
            continue;
        }
        file.seek(SeekFrom::Start(offset.checked_add(relative)?))
            .ok()?;
        let mut signature = [0; 20];
        file.read_exact(&mut signature).ok()?;
        if signature.starts_with(b"Xilinx ISim DBG ")
            && signature[16..19].iter().all(u8::is_ascii_digit)
            && signature[19] == 0
        {
            versions.insert(std::str::from_utf8(&signature[16..19]).ok()?.parse().ok()?);
        }
    }
    (versions.len() == 1).then(|| versions.into_iter().next().unwrap())
}
