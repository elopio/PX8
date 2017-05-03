use std::fs::File;
use std::io::BufReader;
use std::io::Cursor;
use std::io;
use std::io::BufRead;
use std::io::Read;

use std::convert;
use std::fmt;
use std::io::Write;

use std::result::Result;
use std::collections::HashMap;
use std::u32;
use std::str;

use serde_json;

use regex::Regex;

use png;

use gfx;
use gfx::Sprite;

fn read_string(v: &mut Vec<u8>, size: usize) -> String {
    let u: Vec<_> = v.drain(0..size * 2).collect();

    let z = vec![(*u.get(1).unwrap() << 4) | *u.get(0).unwrap(),
                 (*u.get(3).unwrap() << 4) | *u.get(2).unwrap(),
                 (*u.get(5).unwrap() << 4) | *u.get(4).unwrap(),
                 (*u.get(7).unwrap() << 4) | *u.get(6).unwrap()];

    return str::from_utf8(&z).unwrap().to_string();
}

fn read_u8(v: &mut Vec<u8>) -> usize {
    let u: Vec<_> = v.drain(0..2).collect();

    return ((*u.get(1).unwrap() as usize) << 4) | *u.get(0).unwrap() as usize;
}

fn read_u16(v: &mut Vec<u8>) -> usize {
    let u: Vec<_> = v.drain(0..4).collect();

    let x1 = ((*u.get(1).unwrap() as usize) << 4) | *u.get(0).unwrap() as usize;
    let x2 = ((*u.get(3).unwrap() as usize) << 4) | *u.get(2).unwrap() as usize;

    return x1 << 8 | x2;
}

pub struct CartridgePython{
    pub data: String,
}

impl CartridgePython {
    pub fn new(lines: Vec<String>) -> CartridgePython {
        let mut data = "".to_string();

        for line in lines {
            let mut line = line;

            line.push('\n');
            data = data + &line;
        }

        CartridgePython { data: data.clone() }
    }
}

pub struct CartridgeLua{
    pub data: String,
}

impl CartridgeLua {
    pub fn new(lines: Vec<String>, pico8_support: bool) -> CartridgeLua {
        let mut data = "".to_string();

        for line in &lines {
            debug!("LUA LINE {:?}", line);

            let mut line = line.to_string();

            if pico8_support {
                //  lua = lua:gsub("(%S+)%s*([%+-%*/%%])=","%1 = %1 %2 ")
                let re = Regex::new(r"(?P<X>\S+)\s*(?P<Z>[\+\*%/-])=").unwrap();
                if re.is_match(&line) {
                    let line_clone = line.clone();
                    let after = re.replace_all(&line_clone, "$X = $X $Z $Y");
                    debug!("MODIFY {:?} \t=> {:?}", line_clone, after);

                    line.clear();
                    line.push_str(&after);
                }

                let re = Regex::new(r"!=").unwrap();
                if re.is_match(&line) {
                    let line_clone = line.clone();
                    let after = re.replace_all(&line_clone, "~=");
                    debug!("MODIFY {:?} \t=> {:?}", line_clone, after);

                    line.clear();
                    line.push_str(&after);
                }

                let re = Regex::new(r"local function _draw").unwrap();
                if re.is_match(&line) {
                    let line_clone = line.clone();
                    let after = re.replace_all(&line_clone, "function _draw");
                    debug!("MODIFY {:?} \t=> {:?}", line_clone, after);

                    line.clear();
                    line.push_str(&after);
                }

                let re = Regex::new(r"local function _update").unwrap();
                if re.is_match(&line) {
                    let line_clone = line.clone();
                    let after = re.replace_all(&line_clone, "function _update");
                    debug!("MODIFY {:?} \t=> {:?}", line_clone, after);

                    line.clear();
                    line.push_str(&after);
                }

                let re = Regex::new(r"function _update60\(\)").unwrap();
                if re.is_match(&line) {
                    let line_clone = line.clone();
                    let after = re.replace_all(&line_clone, "function _update()");
                    debug!("MODIFY {:?} \t=> {:?}", line_clone, after);

                    line.clear();
                    line.push_str(&after);
                }

                let re = Regex::new(r"if\(_update60").unwrap();
                if re.is_match(&line) {
                    debug!("REMOVE update60");

                    line.clear();
                }

                //  lua = lua:gsub('if%s*(%b())%s*([^\n]*)\n',function(a,b)

                //		local nl = a:find('\n')
                //local th = b:find('%f[%w]then%f[%W]')
                //local an = b:find('%f[%w]and%f[%W]')
                //local o = b:find('%f[%w]or%f[%W]')
                //if nl or th or an or o then
                //return string.format('if %s %s\n',a,b)
                //else
                //return "if "..a:sub(2,#a-1).." then "..b.." end\n"
                //end

                /*let re = Regex::new(r"if\s*\((?P<X>.*)\)(?P<Y>[^\n]*)").unwrap();
            if re.is_match(&line) {
                let re_then = Regex::new(r"then").unwrap();
                if !re_then.is_match(&line) {
                    println!("MATCH {:?}", line);
                    let after = re.replace_all(&line, "if $X then $Y end\n");
                    println!("\t=> {:?}", after);


                    line.clear();
                    line.push_str(&after);
                }
            }*/
            }

            line.push('\n');
            data = data + &line;
        }

        CartridgeLua { data: data.clone() }
    }

    pub fn new_from_bytes(mut v: &mut Vec<u8>, version: u8, pico8_support: bool) -> CartridgeLua {
        info!("CartridgeLua::new_from_bytes");

        let mut vec_code = Vec::new();

        if version == 0 {
            let size_code = v.len();
            debug!("CODE SIZE {:?}", size_code);
            let code_raw: Vec<_> = v.drain(0..(size_code as usize)).collect();

            let mut code: Vec<char> = Vec::new();

            let mut idx: usize = 0;

            while code.len() < size_code as usize {
                let value = (*code_raw.get(idx + 1).unwrap() << 4) | (*code_raw.get(idx).unwrap());
                if value == 0 {
                    break;
                }
                code.push(value as char);
                idx += 2;
            }


            let code_str: String;
            code_str = code.into_iter().collect();

            let lines = code_str.lines();
            for line in lines {
                vec_code.push(line.to_string());
            }
        }

        if version > 0 {
            let version_code = read_string(&mut v, 4);
            let size_code = read_u16(&mut v);

            debug!("VERSION_CODE {:?}", version_code);
            info!("CODE SIZE {:?} [{:?}]", size_code, v.len());

            let unknown_data: Vec<_> = v.drain(0..4).collect();
            debug!("UNKNOWN DATA {:?}", unknown_data);

            let mut size_code_raw = size_code * 2;
            if size_code > v.len() || size_code_raw > v.len() {
                size_code_raw = v.len();
            }

            let code_raw: Vec<_> = v.drain(0..size_code_raw as usize).collect();
            debug!("CODE RAW {:?}", code_raw);
            let mut code: Vec<char> = Vec::new();

            let mut idx: usize = 0;

            let map_converter = "#\n 0123456789abcdefghijklmnopqrstuvwxyz!#%(){}[]<>+=/*:;.,~_".to_string().into_bytes();

            while code.len() < size_code as usize {
                let x = (*code_raw.get(idx + 1).unwrap() << 4) | (*code_raw.get(idx).unwrap());

                debug!("CURRENT {:?} {:?} ({:?} < {:?})", x, idx, code.len(), size_code);

                if x == 0x0 {
                    let value = (*code_raw.get(idx + 2 + 1).unwrap() << 4) | (*code_raw.get(idx + 2).unwrap());
                    code.push(value as char);
                    idx += 2;
                } else if x <= 0x3b {
                    let value = *map_converter.get(x as usize).unwrap() as char;
                    code.push(value);
                } else {
                    let code1: usize = x as usize;
                    let code2: usize = (*code_raw.get(idx + 2 + 1).unwrap() << 4) as usize | (*code_raw.get(idx + 2).unwrap()) as usize;
                    idx += 2;


                    let byte_ago: usize = ((code1 - 0x3C) * 16 + (code2 & 0xF)) as usize;
                    let size: usize = ((code2 >> 4) + 2) as usize;

                    debug!("IDX:{:?} AGO:{:?} SIZE:{:?}", idx, byte_ago, size);

                    let mut u: Vec<char> = Vec::new();
                    let mut idx2 = code.len() - byte_ago;

                    while idx2 < code.len() - byte_ago + size {
                        u.push(*code.get(idx2).unwrap());
                        idx2 += 1;
                    }

                    for n in &u {
                        code.push(*n);
                    }
                }

                idx += 2;
            }

            debug!("Collect code data");

            let code_str: String;

            code_str = code.into_iter().collect();

            let lines = code_str.lines();
            for line in lines {
                vec_code.push(line.to_string());
            }
        }

        return CartridgeLua::new(vec_code, pico8_support);
    }
}

pub struct CartridgeCode {
    pub raw: bool,
    pub lines: Vec<String>,
    pub data: Vec<u8>,
    pub version: u8,
    pub code_type: String,
    pub filename: String,
    pub mode: bool,
}

impl CartridgeCode {
    pub fn new(code_type: String, lines: &mut Vec<String>) -> CartridgeCode {
        info!("[CARTRIDGE] CartridgeCode");

        CartridgeCode {
            raw: false,
            lines: lines.clone(),
            data: Vec::new(),
            version: 0,
            code_type: code_type,
            filename: "".to_string(),
            mode: false
        }
    }

    pub fn new_from_bytes(code_type: String, data: &mut Vec<u8>, version: u8) -> CartridgeCode {
        info!("[CARTRIDGE] CartridgeCode");

        CartridgeCode {
            raw: true,
            lines: Vec::new(),
            data: data.clone(),
            version: version,
            code_type: code_type,
            filename: "".to_string(),
            mode: false
        }
    }

    pub fn set_filename(&mut self, filename: String) {
        self.filename = filename;
    }

    pub fn get_name(&mut self) -> String {
        self.code_type.clone()
    }

    pub fn reload(&mut self) {
        let f1 = File::open(self.filename.clone()).unwrap();
        let buf_reader = BufReader::new(f1);

        let mut code_section = Vec::new();

        for line in buf_reader.lines() {
            let l = line.unwrap();
            code_section.push(l);
        }

        self.lines = code_section.clone();
    }

    pub fn get_data(&mut self) -> String {
        if self.code_type == "lua" {
            if self.raw {
                let cart = CartridgeLua::new_from_bytes(&mut self.data, self.version, self.mode);
                return cart.data;
            } else {
                let cart = CartridgeLua::new(self.lines.clone(), self.mode);
                return cart.data;
            }
        }

        if self.code_type == "python" {
            if !self.raw {
                let cart = CartridgePython::new(self.lines.clone());
                return cart.data;
            }
        }

        return "".to_string();
    }
}

pub struct CartridgeGFX {
    pub sprites: Vec<Sprite>,
}

impl fmt::Debug for CartridgeGFX {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let comma_separated = String::new();
        write!(f,
               "CartridgeGFX {{ sprites: {} {} }}",
               self.sprites.len(),
               comma_separated)
    }
}


impl CartridgeGFX {
    pub fn empty() -> CartridgeGFX {
        CartridgeGFX {
            sprites: Vec::new(),
        }
    }

    pub fn new(lines: &mut Vec<String>) -> CartridgeGFX {
        info!("[CARTRIDGE] CartridgeGFX");

        let mut sprites: Vec<Sprite> = Vec::new();

        if lines.len() > 0 {
            let mut v = Vec::new();

            for line in lines {
                if line.len() > 128 {
                    continue;
                }


                for c in line.as_bytes() {
                    v.push((*c as char).to_digit(16).unwrap());
                }
            }


            let mut g_off = 0;
            // Fill all sprites
            for idx in 0..256 {
                let mut data: [u8; 8 * 8] = [0; 8 * 8];

                let mut idx_vec = 0;

                if idx > 0 {
                    if idx % 16 == 0 {
                        g_off = idx * 8 * 8;
                    } else {
                        g_off += 8;
                    }
                }

                for y in 0..8 {
                    for x in 0..8 {
                        let offset = g_off + y * 128 + x;

                        data[idx_vec] = v[offset] as u8;
                        idx_vec += 1;
                    }
                }

                sprites.push(Sprite::new(data));
            }
        }


        CartridgeGFX { sprites: sprites }
    }

    pub fn new_from_bytes(v: Vec<u8>) -> CartridgeGFX {
        info!("[CARTRIDGE] CartridgeGFX");

        let mut sprites: Vec<Sprite> = Vec::new();

        if v.len() > 0 {
            let mut g_off = 0;
            // Fill all sprites
            for idx in 0..256 {
                let mut data: [u8; 8 * 8] = [0; 8 * 8];

                let mut idx_vec = 0;

                if idx > 0 {
                    if idx % 16 == 0 {
                        g_off = idx * 8 * 8;
                    } else {
                        g_off += 8;
                    }
                }

                for y in 0..8 {
                    for x in 0..8 {
                        let offset = g_off + y * 128 + x;

                        data[idx_vec] = v[offset] as u8;
                        idx_vec += 1;
                    }
                }

                sprites.push(Sprite::new(data));
            }
        }

        CartridgeGFX { sprites: sprites }
    }

    pub fn set_sprites(&mut self, sprites: Vec<Sprite>) {
        self.sprites = sprites;
    }

    pub fn get_data(&mut self) -> String {

        let mut data = String::new();

        let mut idx_sprites = 0;
        let mut line;

        for y in 0..128 {
            line = y % 8;

            if y > 0 && (y % 8) == 0 {
                idx_sprites = idx_sprites + 16;
            }

            for idx in idx_sprites..idx_sprites+16 {
                let mut gfx_sprites = self.sprites[idx].clone();

                data.push_str(&gfx_sprites.get_line(line));
            }

            data.push('\n');
        }

        return data;
    }
}

pub struct CartridgeGFF {
    pub flags: Vec<u8>,
}

impl CartridgeGFF {
    pub fn empty() -> CartridgeGFF {
        CartridgeGFF {
            flags: Vec::new(),
        }
    }

    pub fn new(lines: &mut Vec<String>) -> CartridgeGFF {
        info!("[CARTRIDGE] CartridgeGFF");

        let mut v = Vec::new();

        for line in lines {
            for c in line.as_bytes() {
                v.push((*c as char).to_digit(16).unwrap() as u8);
            }
        }

        let mut v_order = Vec::new();
        let mut idx = 0;
        while idx < v.len() {
            v_order.push(v[idx+1]);
            v_order.push(v[idx]);

            idx += 2;
        }

        CartridgeGFF::new_from_bytes(v_order.clone())
    }

    pub fn new_from_bytes(v: Vec<u8>) -> CartridgeGFF {
        let mut flags: Vec<u8> = Vec::new();

        let mut v_copy = v.clone();

        let len_v = v_copy.len();
        let mut idx: usize = 0;

        while idx < len_v {

            let flag = read_u8(&mut v_copy);

            flags.push(flag as u8);

            idx += 2;
        }

        CartridgeGFF {
            flags: flags.clone(),
        }
    }

    pub fn get_data(&mut self) -> String {
        let mut data = String::new();

        let mut idx_sprites = 0;

        for flag in &self.flags {
            data.push_str(&format!("{:x}{:x}", (flag & 0xf0) >> 4, flag & 0x0f));

            idx_sprites += 1;

            if idx_sprites > 0 && idx_sprites % 128 == 0 {
                data.push('\n');
            }
        }

        return data;
    }
}

pub struct CartridgeMusic {}

impl CartridgeMusic {
    pub fn new(_lines: &mut Vec<String>) -> CartridgeMusic {
        info!("[CARTRIDGE] CartridgeMusic");
        CartridgeMusic {}
    }

    pub fn new_from_bytes(_v: Vec<u8>) -> CartridgeMusic {
        CartridgeMusic {}
    }

    pub fn empty() -> CartridgeMusic {
        CartridgeMusic {}
    }
}

pub struct CartridgeMap {
    pub map: [[u32; 32]; gfx::SCREEN_WIDTH],
}

impl CartridgeMap {
    pub fn empty() -> CartridgeMap {
        CartridgeMap { map: [[0; 32]; gfx::SCREEN_WIDTH] }
    }

    pub fn new(lines: &mut Vec<String>) -> CartridgeMap {
        info!("[CARTRIDGE] CartridgeMap");

        let mut map: [[u32; 32]; gfx::SCREEN_WIDTH] = [[0; 32]; gfx::SCREEN_WIDTH];
        let mut x;
        let mut y = 0;

        for line in lines {
            let mut i = 0;
            x = 0;

            while i < 256 {
                let v: String = line.drain(0..2).collect();
                let z = u32::from_str_radix(&v, 16).unwrap();
                //info!("VAL {:?} {:?}", v, z);

                map[x][y] = z;

                i += 2;
                x += 1;
            }

            y += 1;

            if y == 32 {
                break;
            }
        }


        CartridgeMap { map: map }
    }

    pub fn new_from_bytes(v: Vec<u8>) -> CartridgeMap {
        let mut map: [[u32; 32]; gfx::SCREEN_WIDTH] = [[0; 32]; gfx::SCREEN_WIDTH];

        let mut idx_x;
        let mut idx_y = 0;
        let mut idx = 0;

        while idx_y < 32 {
            idx_x = 0;
            while idx_x < 128 {
                let value = (*v.get(idx + 1).unwrap() << 4) | (*v.get(idx).unwrap());

                map[idx_x][idx_y] = value as u32;

                idx += 2;
                idx_x += 1;
            }

            idx_y += 1;
        }

        CartridgeMap { map: map }
    }

    pub fn get_data(&mut self) -> String {
        let mut data = String::new();

        for y in 0..32 {
            for x in 0..128 {
                data.push_str(&format!("{:02x}", self.map[x][y]));
            }
            data.push('\n');
        }

        return data;
    }
}

pub enum CartridgeFormat {
    PngFormat = 0,
    P8Format = 1,
    Px8Format = 2,
}

pub struct Cartridge {
    pub filename: String,
    pub data_filename: String,
    pub header: String,
    pub version: String,
    pub gfx: CartridgeGFX,
    pub map: CartridgeMap,
    pub gff: CartridgeGFF,
    pub code: CartridgeCode,
    pub music: CartridgeMusic,
    pub format: CartridgeFormat,
    pub edit: bool,
}

pub static SECTION_DELIM_RE: &'static str = r"^__(\w+)__$";

#[derive(Debug)]
pub enum Error {
    Err(String),
    IOError(io::Error),
}

impl convert::From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::IOError(e)
    }
}

fn read_from_pngformat<R: io::BufRead>(filename: String, buf: &mut R) -> Result<Cartridge, Error> {
    let decoder = png::Decoder::new(buf);
    let (info, mut reader) = decoder.read_info().unwrap();

    let mut buf = vec![0; info.buffer_size()];
    let mut picodata = Vec::new();

    reader.next_frame(&mut buf).unwrap();

    let mut row = 0;
    while row < buf.len() {
        for col_i in 0..info.width {
            let g_idx: u32 = row as u32;

            let mut r: u8 = *buf.get((g_idx + col_i * 4 + 0) as usize).unwrap() as u8;
            let mut g: u8 = *buf.get((g_idx + col_i * 4 + 1) as usize).unwrap() as u8;
            let mut b: u8 = *buf.get((g_idx + col_i * 4 + 2) as usize).unwrap() as u8;
            let mut a: u8 = *buf.get((g_idx + col_i * 4 + 3) as usize).unwrap() as u8;

            r = r & 3;
            g = g & 3;
            b = b & 3;
            a = a & 3;

            let v = b | (g << 2) | (r << 4) | (a << 6);
            let lo = v & 0x0f;
            let hi = v >> 4;

            picodata.push(lo);
            picodata.push(hi);
        }

        row = row + 640;
    }

    let mut gfx_data = Vec::new();
    for i in 0..0x2000 * 2 {
        gfx_data.push(*picodata.get(i).unwrap());
    }

    let mut map_data = Vec::new();
    for i in 0x2000 * 2..0x3000 * 2 {
        map_data.push(*picodata.get(i).unwrap());
    }

    let mut gff_data = Vec::new();
    for i in 0x3000 * 2..0x3100 * 2 {
        gff_data.push(*picodata.get(i).unwrap());
    }

    let version = *picodata.get(0x8000 * 2).unwrap();

    let mut code_data = Vec::new();
    for i in 0x4300 * 2..0x8000 * 2 {
        code_data.push(*picodata.get(i).unwrap());
    }

    let mut music_data = Vec::new();
    for i in 0x3100 * 2..0x3200 * 2 {
        music_data.push(*picodata.get(i).unwrap());
    }

    let cartridge_gfx = CartridgeGFX::new_from_bytes(gfx_data);
    let cartridge_code = CartridgeCode::new_from_bytes("lua".to_string(), &mut code_data, version);
    let cartridge_map = CartridgeMap::new_from_bytes(map_data);
    let cartridge_gff = CartridgeGFF::new_from_bytes(gff_data);
    let cartridge_music = CartridgeMusic::new_from_bytes(music_data);

    Ok(Cartridge {
        filename: filename.clone(),
        data_filename: "".to_string(),
        header: "".to_string(),
        version: "".to_string(),
        gfx: cartridge_gfx,
        code: cartridge_code,
        map: cartridge_map,
        gff: cartridge_gff,
        music: cartridge_music,
        format: CartridgeFormat::PngFormat,
        edit: false,
    })
}

fn read_from_p8format<R: io::BufRead>(filename: String, buf: &mut R) -> Result<Cartridge, Error> {
    let mut header = String::new();
    try!(buf.read_line(&mut header));

    let mut version = String::new();
    try!(buf.read_line(&mut version));

    let re_delim_section = Regex::new(SECTION_DELIM_RE).unwrap();

    let mut sections: HashMap<String, Vec<(String)>> = HashMap::new();

    let mut section_name = "".to_string();

    let mut new_section;

    for line in buf.lines() {
        let l = line.unwrap();
        if re_delim_section.is_match(l.as_str()) {
            debug!("NEW SECTION {:?}", l);
            section_name = l.clone();

            let vec_section = Vec::new();
            sections.insert(section_name.clone(), vec_section);
            new_section = false;
        } else {
            new_section = true;
        }

        if new_section {
            match sections.get_mut(&section_name) {
                Some(vec_section2) => vec_section2.push(l),
                _ => debug!("Impossible to find section {:?}", section_name),
            }
        }
    }

    for (section_name, section) in &sections {
        debug!("{}: \"{}\"", section_name, section.len());
    }

    let cartridge_gfx;
    let cartridge_code;
    let cartridge_map;
    let cartridge_gff;
    let cartridge_music;


    if sections.contains_key("__lua__") {
        cartridge_code = CartridgeCode::new("lua".to_string(), sections.get_mut("__lua__").unwrap());
    } else if sections.contains_key("__python__") {
        cartridge_code = CartridgeCode::new("python".to_string(), sections.get_mut("__python__").unwrap());
    } else {
        return Err(Error::Err("NO CODE DATA".to_string()));
    }

    match sections.get_mut("__gfx__") {
        Some(vec_section) => cartridge_gfx = CartridgeGFX::new(vec_section),
        _ => cartridge_gfx = CartridgeGFX::empty(),
    }

    match sections.get_mut("__map__") {
        Some(vec_section) => cartridge_map = CartridgeMap::new(vec_section),
        _ => cartridge_map = CartridgeMap::empty(),
    }

    match sections.get_mut("__gff__") {
        Some(vec_section) => cartridge_gff = CartridgeGFF::new(vec_section),
        _ => cartridge_gff = CartridgeGFF::empty(),
    }

    match sections.get_mut("__music__") {
        Some(vec_section) => cartridge_music = CartridgeMusic::new(vec_section),
        _ => cartridge_music = CartridgeMusic::empty(),
    }


    Ok(Cartridge {
        filename: filename.clone(),
        data_filename: "".to_string(),
        header: header.clone(),
        version: version.clone(),
        gfx: cartridge_gfx,
        code: cartridge_code,
        map: cartridge_map,
        gff: cartridge_gff,
        music: cartridge_music,
        format: CartridgeFormat::P8Format,
        edit: false,
    })
}

#[derive(Serialize, Deserialize)]
struct PX8Format {
    code: String,
    data: String,
}


impl Cartridge {
    #[allow(dead_code)]
    pub fn from_png_raw(filename: String, data: Vec<u8>) -> Result<Cartridge, Error> {
        let mut buf_reader = Cursor::new(data);
        let cartridge = try!(read_from_pngformat(filename.clone(), &mut buf_reader));
        Ok(cartridge)
    }

    pub fn from_png_file(filename: String) -> Result<Cartridge, Error> {
        let f = try!(File::open(filename.clone()));
        let mut buf_reader = BufReader::new(f);
        let cartridge = try!(read_from_pngformat(filename.clone(), &mut buf_reader));
        Ok(cartridge)
    }

    #[allow(dead_code)]
    pub fn from_p8_raw(filename: String, data: Vec<u8>) -> Result<Cartridge, Error> {
        let mut buf_reader = Cursor::new(data);
        let cartridge = try!(read_from_p8format(filename.clone(), &mut buf_reader));
        Ok(cartridge)
    }

    pub fn from_p8_file(filename: String) -> Result<Cartridge, Error> {
        let f = try!(File::open(filename.clone()));
        let mut buf_reader = BufReader::new(f);
        let cartridge = try!(read_from_p8format(filename.clone(), &mut buf_reader));
        Ok(cartridge)
    }

    pub fn from_px8_file(filename: String) -> Result<Cartridge, Error> {
        let mut f = try!(File::open(filename.clone()));

        let mut data = String::new();
        f.read_to_string(&mut data).unwrap();

        let json:PX8Format = serde_json::from_str(&data).unwrap();

        let file_code = json.code.clone();

        let f1 = try!(File::open(file_code.clone()));
        let buf_reader = BufReader::new(f1);

        let mut code_section = Vec::new();

        for line in buf_reader.lines() {
            let l = line.unwrap();
            code_section.push(l);
        }

        let file_data = json.data.clone();
        let f2 = try!(File::open(file_data.clone()));
        let buf_reader = BufReader::new(f2);

        let re_delim_section = Regex::new(SECTION_DELIM_RE).unwrap();

        let mut sections: HashMap<String, Vec<(String)>> = HashMap::new();

        let mut section_name = "".to_string();

        let mut new_section;

        for line in buf_reader.lines() {
            let l = line.unwrap();
            if re_delim_section.is_match(l.as_str()) {
                debug!("NEW SECTION {:?}", l);
                section_name = l.clone();

                let vec_section = Vec::new();
                sections.insert(section_name.clone(), vec_section);
                new_section = false;
            } else {
                new_section = true;
            }

            if new_section {
                match sections.get_mut(&section_name) {
                    Some(vec_section2) => vec_section2.push(l),
                    _ => debug!("Impossible to find section {:?}", section_name),
                }
            }
        }

        for (section_name, section) in &sections {
            debug!("{}: \"{}\"", section_name, section.len());
        }

        let cartridge_gfx;
        let cartridge_gff;
        let mut cartridge_code;
        let cartridge_map;
        let cartridge_music;

        if file_code.contains(".py") {
            cartridge_code = CartridgeCode::new("python".to_string(), &mut code_section);
        } else if file_code.contains(".lua") {
            cartridge_code = CartridgeCode::new("lua".to_string(), &mut code_section);
        } else {
            panic!("Unknown file to load the code {:?}", file_code);
        }

        cartridge_code.set_filename(file_code.to_string());

        match sections.get_mut("__gfx__") {
            Some(vec_section) => cartridge_gfx = CartridgeGFX::new(vec_section),
            _ => cartridge_gfx = CartridgeGFX::empty(),
        }

        match sections.get_mut("__map__") {
            Some(vec_section) => cartridge_map = CartridgeMap::new(vec_section),
            _ => cartridge_map = CartridgeMap::empty(),
        }

        match sections.get_mut("__gff__") {
            Some(vec_section) => cartridge_gff = CartridgeGFF::new(vec_section),
            _ => cartridge_gff = CartridgeGFF::empty(),
        }

        match sections.get_mut("__music__") {
            Some(vec_section) => cartridge_music = CartridgeMusic::new(vec_section),
            _ => cartridge_music = CartridgeMusic::empty(),
        }


        Ok(Cartridge {
            filename: filename.clone(),
            data_filename: file_data.to_string(),
            header: "".to_string(),
            version: "".to_string(),
            gfx: cartridge_gfx,
            code: cartridge_code,
            map: cartridge_map,
            gff: cartridge_gff,
            music: cartridge_music,
            format: CartridgeFormat::Px8Format,
            edit: false,
        })
    }

    pub fn set_mode(&mut self, mode: bool) {
        self.code.mode = mode;
    }


    pub fn save_in_p8(&mut self, filename: String) {
        info!("Save the modified cartridge in P8 format {:?}", filename);

        let mut f = File::create(filename).unwrap();

        match self.format {
            CartridgeFormat::P8Format => {
                f.write_all(self.header.as_bytes()).unwrap();
                f.write_all(self.version.as_bytes()).unwrap();
            },
            _ => {
                f.write_all("Saved by PX8\n".as_bytes()).unwrap();
                f.write_all("Version 0.0.3\n".as_bytes()).unwrap();
            }
        }


        f.write_all(format!("__{:}__\n", self.code.code_type).as_bytes()).unwrap();
        f.write_all(self.code.get_data().clone().as_bytes()).unwrap();

        f.write_all("__gfx__\n".as_bytes()).unwrap();
        f.write_all(self.gfx.get_data().clone().as_bytes()).unwrap();

        f.write_all("__gff__\n".as_bytes()).unwrap();
        f.write_all(self.gff.get_data().clone().as_bytes()).unwrap();

        f.write_all("__map__\n".as_bytes()).unwrap();
        f.write_all(self.map.get_data().clone().as_bytes()).unwrap();

        f.write_all("__sfx__\n".as_bytes()).unwrap();

        f.write_all("__music__\n".as_bytes()).unwrap();
    }

    pub fn save_data(&mut self) {
        info!("Save the data in {:?}", self.data_filename);

        match self.format {
            CartridgeFormat::Px8Format => {
                let mut f = File::create(self.data_filename.clone()).unwrap();

                f.write_all("__gfx__\n".as_bytes()).unwrap();
                f.write_all(self.gfx.get_data().clone().as_bytes()).unwrap();

                f.write_all("__gff__\n".as_bytes()).unwrap();

                f.write_all("__map__\n".as_bytes()).unwrap();
                f.write_all(self.map.get_data().clone().as_bytes()).unwrap();

                f.write_all("__sfx__\n".as_bytes()).unwrap();

                f.write_all("__music__\n".as_bytes()).unwrap();
            },
            _ => ()
        }
    }


    pub fn dump(&mut self, filename: String) {
        info!("Dump the code in {:?}", filename);

        let mut f = File::create(filename).unwrap();
        f.write_all(self.code.get_data().clone().as_bytes()).unwrap();
    }
}

impl fmt::Debug for Cartridge {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
               "Cartridge {{ filename: {}, gfx: {:?} }}",
               self.filename,
               self.gfx)
    }
}