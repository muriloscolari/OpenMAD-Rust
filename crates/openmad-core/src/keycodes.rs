//! QMK keycode names, categories, and keymap buffer decoding.

pub fn label(keycode: u16) -> String {
    if let Some(s) = basic_name(keycode) {
        return s.to_string();
    }
    if let Some(s) = system_name(keycode) {
        return s.to_string();
    }
    if (0x7700..=0x770F).contains(&keycode) {
        return format!("M{}", keycode - 0x7700);
    }
    let layer_ranges = [
        (0x5200u16, "TO"),
        (0x5220u16, "MO"),
        (0x5240u16, "DF"),
        (0x5260u16, "TG"),
        (0x5280u16, "OSL"),
    ];
    for (base, prefix) in layer_ranges {
        if (base..=base + 0x1F).contains(&keycode) {
            return format!("{prefix}({})", keycode - base);
        }
    }
    format!("{keycode:#06x}")
}

pub fn basic_name(kc: u16) -> Option<&'static str> {
    Some(match kc {
        0x0000 => "----",
        0x0001 => "XXXX",
        0x0004 => "A",
        0x0005 => "B",
        0x0006 => "C",
        0x0007 => "D",
        0x0008 => "E",
        0x0009 => "F",
        0x000A => "G",
        0x000B => "H",
        0x000C => "I",
        0x000D => "J",
        0x000E => "K",
        0x000F => "L",
        0x0010 => "M",
        0x0011 => "N",
        0x0012 => "O",
        0x0013 => "P",
        0x0014 => "Q",
        0x0015 => "R",
        0x0016 => "S",
        0x0017 => "T",
        0x0018 => "U",
        0x0019 => "V",
        0x001A => "W",
        0x001B => "X",
        0x001C => "Y",
        0x001D => "Z",
        0x001E => "1",
        0x001F => "2",
        0x0020 => "3",
        0x0021 => "4",
        0x0022 => "5",
        0x0023 => "6",
        0x0024 => "7",
        0x0025 => "8",
        0x0026 => "9",
        0x0027 => "0",
        0x0028 => "Enter",
        0x0029 => "Esc",
        0x002A => "Bspc",
        0x002B => "Tab",
        0x002C => "Space",
        0x002D => "-",
        0x002E => "=",
        0x002F => "[",
        0x0030 => "]",
        0x0031 => "\\",
        0x0032 => "#",
        0x0033 => ";",
        0x0034 => "'",
        0x0035 => "`",
        0x0036 => ",",
        0x0037 => ".",
        0x0038 => "/",
        0x0039 => "Caps",
        0x003A => "F1",
        0x003B => "F2",
        0x003C => "F3",
        0x003D => "F4",
        0x003E => "F5",
        0x003F => "F6",
        0x0040 => "F7",
        0x0041 => "F8",
        0x0042 => "F9",
        0x0043 => "F10",
        0x0044 => "F11",
        0x0045 => "F12",
        0x0046 => "PrtSc",
        0x0047 => "ScrLk",
        0x0048 => "Pause",
        0x0049 => "Ins",
        0x004A => "Home",
        0x004B => "PgUp",
        0x004C => "Del",
        0x004D => "End",
        0x004E => "PgDn",
        0x004F => "Rgt",
        0x0050 => "Lft",
        0x0051 => "Dn",
        0x0052 => "Up",
        0x0053 => "NumLk",
        0x0054 => "KP/",
        0x0055 => "KP*",
        0x0056 => "KP-",
        0x0057 => "KP+",
        0x0058 => "KPEnt",
        0x0059 => "KP1",
        0x005A => "KP2",
        0x005B => "KP3",
        0x005C => "KP4",
        0x005D => "KP5",
        0x005E => "KP6",
        0x005F => "KP7",
        0x0060 => "KP8",
        0x0061 => "KP9",
        0x0062 => "KP0",
        0x0063 => "KP.",
        0x0064 => "NUBS",
        0x0065 => "App",
        0x0068 => "F13",
        0x0069 => "F14",
        0x006A => "F15",
        0x006B => "F16",
        0x006C => "F17",
        0x006D => "F18",
        0x006E => "F19",
        0x006F => "F20",
        0x0070 => "F21",
        0x0071 => "F22",
        0x0072 => "F23",
        0x0073 => "F24",
        0x00E0 => "LCtl",
        0x00E1 => "LSft",
        0x00E2 => "LAlt",
        0x00E3 => "LGui",
        0x00E4 => "RCtl",
        0x00E5 => "RSft",
        0x00E6 => "RAlt",
        0x00E7 => "RGui",
        _ => return None,
    })
}

pub fn system_name(kc: u16) -> Option<&'static str> {
    Some(match kc {
        0x00A5 => "Power",
        0x00A6 => "Sleep",
        0x00A7 => "Wake",
        0x00A8 => "Mute",
        0x00A9 => "Vol+",
        0x00AA => "Vol-",
        0x00AB => "Next",
        0x00AC => "Prev",
        0x00AD => "Stop",
        0x00AE => "Play",
        0x00AF => "Sel",
        0x00B0 => "Eject",
        0x00B1 => "Mail",
        0x00B2 => "Calc",
        0x00B3 => "MyPC",
        0x00B4 => "Search",
        0x00B5 => "WWW",
        0x00B6 => "Back",
        0x00B7 => "Fwd",
        0x00B8 => "Refresh",
        0x00B9 => "Favs",
        0x00BD => "Bri+",
        0x00BE => "Bri-",
        _ => return None,
    })
}

pub const KEYBOARD_PICKER_CODES: &[(u16, &str)] = &[
    (0x0000, "None"),
    (0x0001, "Trans"),
    (0x0029, "Esc"),
    (0x003A, "F1"), (0x003B, "F2"), (0x003C, "F3"), (0x003D, "F4"),
    (0x003E, "F5"), (0x003F, "F6"), (0x0040, "F7"), (0x0041, "F8"),
    (0x0042, "F9"), (0x0043, "F10"), (0x0044, "F11"), (0x0045, "F12"),
    (0x001E, "1"), (0x001F, "2"), (0x0020, "3"), (0x0021, "4"), (0x0022, "5"),
    (0x0023, "6"), (0x0024, "7"), (0x0025, "8"), (0x0026, "9"), (0x0027, "0"),
    (0x0004, "A"), (0x0005, "B"), (0x0006, "C"), (0x0007, "D"), (0x0008, "E"),
    (0x0009, "F"), (0x000A, "G"), (0x000B, "H"), (0x000C, "I"), (0x000D, "J"),
    (0x000E, "K"), (0x000F, "L"), (0x0010, "M"), (0x0011, "N"), (0x0012, "O"),
    (0x0013, "P"), (0x0014, "Q"), (0x0015, "R"), (0x0016, "S"), (0x0017, "T"),
    (0x0018, "U"), (0x0019, "V"), (0x001A, "W"), (0x001B, "X"), (0x001C, "Y"),
    (0x001D, "Z"),
    (0x002B, "Tab"), (0x0039, "Caps"), (0x0028, "Enter"), (0x002A, "Bspc"),
    (0x002C, "Space"), (0x002D, "-"), (0x002E, "="), (0x002F, "["), (0x0030, "]"),
    (0x0031, "\\"), (0x0033, ";"), (0x0034, "'"), (0x0035, "`"), (0x0036, ","),
    (0x0037, "."), (0x0038, "/"),
    (0x00E0, "LCtl"), (0x00E1, "LSft"), (0x00E2, "LAlt"), (0x00E3, "LGui"),
    (0x00E4, "RCtl"), (0x00E5, "RSft"), (0x00E6, "RAlt"), (0x00E7, "RGui"),
    (0x0052, "Up"), (0x0051, "Dn"), (0x0050, "Lft"), (0x004F, "Rgt"),
    (0x0049, "Ins"), (0x004C, "Del"), (0x004A, "Home"), (0x004D, "End"),
    (0x004B, "PgUp"), (0x004E, "PgDn"), (0x0046, "PrtSc"), (0x0047, "ScrLk"),
    (0x0048, "Pause"), (0x0065, "App"), (0x0064, "NUBS"),
    (0x0053, "NumLk"), (0x0054, "KP/"), (0x0055, "KP*"), (0x0056, "KP-"),
    (0x0057, "KP+"), (0x0058, "KPEnt"), (0x005F, "KP7"), (0x0060, "KP8"),
    (0x0061, "KP9"), (0x005C, "KP4"), (0x005D, "KP5"), (0x005E, "KP6"),
    (0x0059, "KP1"), (0x005A, "KP2"), (0x005B, "KP3"), (0x0062, "KP0"),
    (0x0063, "KP."),
];

pub const SYSTEM_PICKER_CODES: &[(u16, &str)] = &[
    (0x00A8, "Mute"), (0x00A9, "Vol+"), (0x00AA, "Vol-"),
    (0x00AE, "Play/Pause"), (0x00AD, "Stop"), (0x00AB, "Next"), (0x00AC, "Prev"),
    (0x00BD, "Bri+"), (0x00BE, "Bri-"),
    (0x00B1, "Mail"), (0x00B2, "Calc"), (0x00B3, "MyPC"),
    (0x00B4, "Search"), (0x00B5, "WWW"), (0x00B6, "Back"), (0x00B7, "Fwd"),
    (0x00A5, "Power"), (0x00A6, "Sleep"), (0x00A7, "Wake"),
];

pub const LAYER_PICKER_CODES: &[(u16, &str)] = &[
    (0x5220, "MO(0)"), (0x5221, "MO(1)"), (0x5222, "MO(2)"), (0x5223, "MO(3)"),
    (0x5260, "TG(0)"), (0x5261, "TG(1)"), (0x5262, "TG(2)"), (0x5263, "TG(3)"),
    (0x5200, "TO(0)"), (0x5201, "TO(1)"), (0x5202, "TO(2)"), (0x5203, "TO(3)"),
    (0x5240, "DF(0)"), (0x5241, "DF(1)"), (0x5242, "DF(2)"), (0x5243, "DF(3)"),
    (0x5280, "OSL(0)"), (0x5281, "OSL(1)"), (0x5282, "OSL(2)"), (0x5283, "OSL(3)"),
];

pub fn decode_keymap(keymap: &[u8], layer: usize, rows: usize, cols: usize) -> Vec<Vec<u16>> {
    let mut out = Vec::with_capacity(rows);
    for r in 0..rows {
        let mut row_vec = Vec::with_capacity(cols);
        for c in 0..cols {
            let off = ((layer * rows + r) * cols + c) * 2;
            if off + 1 < keymap.len() {
                row_vec.push(u16::from_be_bytes([keymap[off], keymap[off + 1]]));
            } else {
                row_vec.push(0);
            }
        }
        out.push(row_vec);
    }
    out
}

pub fn decode_all_layers(keymap: &[u8], layers: usize, rows: usize, cols: usize) -> Vec<Vec<Vec<u16>>> {
    (0..layers)
        .map(|l| decode_keymap(keymap, l, rows, cols))
        .collect()
}

pub fn encode_all_layers(grid: &[Vec<Vec<u16>>], layers: usize, rows: usize, cols: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(layers * rows * cols * 2);
    for l in 0..layers {
        for r in 0..rows {
            for c in 0..cols {
                let kc = grid
                    .get(l)
                    .and_then(|lr| lr.get(r))
                    .and_then(|row| row.get(c))
                    .copied()
                    .unwrap_or(0);
                out.extend_from_slice(&kc.to_be_bytes());
            }
        }
    }
    out
}

pub fn keymap_diff_count(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    (0..n)
        .step_by(2)
        .filter(|&i| i + 1 < n && (a[i] != b[i] || a[i + 1] != b[i + 1]))
        .count()
}

pub fn all_searchable_codes() -> Vec<(u16, String)> {
    let mut out: Vec<(u16, String)> = KEYBOARD_PICKER_CODES
        .iter()
        .map(|&(k, lbl)| (k, lbl.to_string()))
        .collect();
    for &(k, lbl) in SYSTEM_PICKER_CODES {
        out.push((k, lbl.to_string()));
    }
    for i in 0..16u16 {
        out.push((0x7700 + i, format!("M{i}")));
    }
    for &(k, lbl) in LAYER_PICKER_CODES {
        out.push((k, lbl.to_string()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_labels_and_keymap_roundtrip() {
        assert_eq!(label(0x0004), "A");
        assert_eq!(label(0x5221), "MO(1)");
        assert_eq!(label(0x7703), "M3");
        assert_eq!(label(0x005F), "KP7");
        assert_eq!(label(0x1234), "0x1234");

        let mut layers = vec![vec![vec![0u16; 14]; 5]; 4];
        layers[0][2][1] = 0x0004;
        layers[1][4][10] = 0x5221;
        let bytes = encode_all_layers(&layers, 4, 5, 14);
        assert_eq!(bytes.len(), 560);
        let decoded = decode_all_layers(&bytes, 4, 5, 14);
        assert_eq!(decoded, layers);
        assert_eq!(keymap_diff_count(&bytes, &bytes), 0);

        let mut bytes2 = bytes.clone();
        bytes2[0] = 0x00;
        bytes2[1] = 0x29;
        assert_eq!(keymap_diff_count(&bytes, &bytes2), 1);
    }
}
