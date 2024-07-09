use super::{registers::{alpha_blend_register::AlphaBlendRegister, bg_control_register::BgControlRegister, brightness_register::BrightnessRegister, color_effects_register::ColorEffectsRegister, display_control_register::{BgMode, DisplayControlRegister, DisplayControlRegisterFlags, DisplayMode}, master_brightness_register::MasterBrightnessRegister, window_horizontal_register::WindowHorizontalRegister, window_in_register::WindowInRegister, window_out_register::WindowOutRegister, window_vertical_register::WindowVerticalRegister}, vram::VRam, BgProps, SCREEN_HEIGHT, SCREEN_WIDTH};

#[derive(Copy, Clone)]
pub struct Color {
  pub r: u8,
  pub g: u8,
  pub b: u8
}

impl Color {
  pub fn from(val: u16) -> Self {
    let mut r = (val & 0x1f) as u8;
    let mut g = ((val >> 5) & 0x1f) as u8;
    let mut b = ((val >> 10) & 0x1f) as u8;

    r = (r << 3) | (r >> 2);
    g = (g << 3) | (g >> 2);
    b = (b << 3) | (b >> 2);

    Self {
      r,
      g,
      b
    }
  }
}

pub struct Engine2d<const IS_ENGINE_B: bool> {
  pub dispcnt: DisplayControlRegister<IS_ENGINE_B>,
  pub oam: [u8; 0x400],
  pub pixels: [u8; 3 * (SCREEN_WIDTH * SCREEN_HEIGHT) as usize],
  pub winin: WindowInRegister,
  pub winout: WindowOutRegister,
  pub winh: [WindowHorizontalRegister; 2],
  pub winv: [WindowVerticalRegister; 2],
  pub bldcnt: ColorEffectsRegister,
  pub bldalpha: AlphaBlendRegister,
  pub bldy: BrightnessRegister,
  pub bgcnt: [BgControlRegister; 4],
  pub bgxofs: [u16; 4],
  pub bgyofs: [u16; 4],
  pub bg_props: [BgProps; 2],
  bg_lines: [[Option<(u8, u8, u8)>; SCREEN_WIDTH as usize]; 4],
  pub master_brightness: MasterBrightnessRegister,
  pub bg_palette_ram: [u8; 0x200],
  pub obj_palette_ram: [u8; 0x200]
}

impl<const IS_ENGINE_B: bool> Engine2d<IS_ENGINE_B> {
  pub fn new() -> Self {
    Self {
      dispcnt: DisplayControlRegister::new(),
      oam: [0; 0x400],
      pixels: [0; 3 * (SCREEN_WIDTH * SCREEN_HEIGHT) as usize],
      bgxofs: [0; 4],
      bgyofs: [0; 4],
      bg_props: [BgProps::new(); 2],
      winh: [WindowHorizontalRegister::new(); 2],
      winv: [WindowVerticalRegister::new(); 2],
      winin: WindowInRegister::from_bits_retain(0),
      winout: WindowOutRegister::from_bits_retain(0),
      bldcnt: ColorEffectsRegister::new(),
      bldalpha: AlphaBlendRegister::new(),
      bldy: BrightnessRegister::new(),
      bgcnt: [BgControlRegister::from_bits_retain(0); 4],
      bg_lines: [[None; SCREEN_WIDTH as usize]; 4],
      master_brightness: MasterBrightnessRegister::new(),
      bg_palette_ram: [0; 0x200],
      obj_palette_ram: [0; 0x200]
    }
  }

  pub fn write_palette_ram(&mut self, address: u32, byte: u8) {
    let mut address = address & (2 * self.bg_palette_ram.len() - 1) as u32;

    let ram = if address < self.bg_palette_ram.len() as u32 {
      &mut self.bg_palette_ram
    } else {
      &mut self.obj_palette_ram
    };

    address = address & (ram.len() - 1) as u32;

    ram[address as usize] = byte;
  }

  pub fn render_normal_line(&mut self, y: u16, vram: &VRam) {
    if self.dispcnt.flags.contains(DisplayControlRegisterFlags::DISPLAY_OBJ) {
      self.render_objects();
    }

    match self.dispcnt.bg_mode {
      BgMode::Mode0 => {
        for i in 0..4 {
          if self.bg_mode_enabled(i) {
            self.render_text_line(i, y, vram);
          }
        }
      }
      BgMode::Mode1 => {
        for i in 0..3 {
          if self.bg_mode_enabled(i) {
            self.render_text_line(i, y, vram);
          }
        }

        if self.bg_mode_enabled(3) {
          self.render_affine_line(3);
        }
      }
      BgMode::Mode2 => {
        for i in 0..2 {
          if self.bg_mode_enabled(i) {
            self.render_text_line(i, y, vram);
          }
        }

        for i in 2..4 {
          if self.bg_mode_enabled(i) {
            self.render_affine_line(i);
          }
        }
      }
      BgMode::Mode3 => {
        for i in 0..3 {
          if self.bg_mode_enabled(i) {
            self.render_text_line(i, y, vram);
          }
        }

        if self.bg_mode_enabled(3) {
          self.render_extended_line(3);
        }
      }
      BgMode::Mode4 => {
        for i in 0..2 {
          if self.bg_mode_enabled(i) {
            self.render_text_line(i, y, vram);
          }
        }

        if self.bg_mode_enabled(2) {
          self.render_affine_line(2);
        }

        if self.bg_mode_enabled(3) {
          self.render_extended_line(3);
        }
      }
      BgMode::Mode5 => {
        for i in 0..2 {
          if self.bg_mode_enabled(i) {
            self.render_text_line(i, y, vram);
          }
        }

        if self.bg_mode_enabled(2) {
          self.render_extended_line(2);
        }

        if self.bg_mode_enabled(3) {
          self.render_extended_line(3);
        }
      }
      BgMode::Mode6 => (), // TODO
      _ => panic!("reserved option given for bg mode: 7")
    }

    self.finalize_scanline();
  }

  fn render_extended_line(&mut self, bg_index: usize) {

  }

  fn finalize_scanline(&mut self) {

  }

  fn render_affine_line(&mut self, bg_index: usize) {

  }
  fn render_objects(&mut self) {

  }

  fn bg_mode_enabled(&self, bg_index: usize) -> bool {
    match bg_index {
      0 => self.dispcnt.flags.contains(DisplayControlRegisterFlags::DISPLAY_BG0),
      1 => self.dispcnt.flags.contains(DisplayControlRegisterFlags::DISPLAY_BG1),
      2 => self.dispcnt.flags.contains(DisplayControlRegisterFlags::DISPLAY_BG2),
      3 => self.dispcnt.flags.contains(DisplayControlRegisterFlags::DISPLAY_BG3),
      _ => unreachable!("can't happen")
    }
  }

  fn render_text_line(&mut self, bg_index: usize, y: u16, vram: &VRam) {
    let (x_offset, y_offset) = (self.bgxofs[bg_index], self.bgyofs[bg_index]);
    /*
      engine A screen base: BGxCNT.bits*2K + DISPCNT.bits*64K
      engine B screen base: BGxCNT.bits*2K + 0
      engine A char base: BGxCNT.bits*16K + DISPCNT.bits*64K
      engine B char base: BGxCNT.bits*16K + 0
     */
    let (tilemap_base, tile_base) = if !IS_ENGINE_B {
      (self.bgcnt[bg_index].screen_base_block() as u32 * 0x800 + self.dispcnt.screen_base * 0x1_0000, self.bgcnt[bg_index].character_base_block() as u32 * 0x4000 + self.dispcnt.character_base * 0x1_0000)
    } else {
      (self.bgcnt[bg_index].screen_base_block() as u32 * 0x800, self.bgcnt[bg_index].character_base_block() as u32 * 0x4000)
    };

    let mut x = 0;

    let x_in_bg = x + x_offset;
    let y_in_bg = y + y_offset;

    let mut x_tile_number = (x_in_bg / 8) % 32;
    let y_tile_number = (x_in_bg / 8) % 32;

    let mut x_pos_in_tile = x_in_bg % 8;
    let y_pos_in_tile = y_in_bg % 8;

    let mut screen_index = match self.bgcnt[bg_index].screen_size() {
      0 => 0,
      1 => x_in_bg / 256, // 512 x 256
      2 => y_in_bg / 256, // 256 x 512
      3 => (x_in_bg / 256) + ((y_in_bg / 256) * 2), // 512 x 512
      _ => unreachable!("not possible")
    };

    let tile_size: u32 = if self.bgcnt[bg_index].contains(BgControlRegister::PALETTES) {
      64
    } else {
      32
    };

    while x < SCREEN_WIDTH {
      let tile_number = x_tile_number + y_tile_number * 32;
      let mut tilemap_address = tilemap_base + 0x800 * screen_index as u32 + 2 * tile_number as u32;

      'outer: for _ in x_tile_number..32 {
        let attributes = if !IS_ENGINE_B {
          vram.read_engine_a_bg(tilemap_address) as u16 | (vram.read_engine_a_bg(tilemap_address) as u16) << 8
        } else {
          0 // TODO
        };

        let x_flip = (attributes >> 10) & 0x1 == 1;
        let y_flip =  (attributes >> 11) & 0x1 == 1;
        let palette_number = (attributes >> 12) & 0xf;
        let tile_number = attributes & 0x3ff;

        let tile_address = tile_base + tile_number as u32 * tile_size as u32;

        for tile_x in x_pos_in_tile..8 {
          let palette_index = if tile_size == 64 {
            self.get_pixel_index_bpp8(tile_address, tile_x, y_pos_in_tile, x_flip, y_flip, vram)
          } else {
            self.get_pixel_index_bpp4(tile_address, tile_x, y_pos_in_tile, x_flip, y_flip, vram)
          };

          let palette_bank = if tile_size == 64 {
            0
          } else {
            palette_number
          };

          println!("x_flip = {x_flip} y_flip = {y_flip} palette_number = {palette_number} tile_number = {tile_number}");

          // self.bg_lines[bg_index][x as usize] = self.get_palette_color(palette_index as usize, palette_bank as usize, 0);

          x += 1;

          if x == SCREEN_WIDTH {
            break 'outer;
          }
        }
        x_pos_in_tile = 0;
        tilemap_address += 2;
      }
    }


  }

  pub fn render_line(&mut self, y: u16, vram: &mut VRam) {
    match self.dispcnt.display_mode {
      DisplayMode::Mode0 => {
        let color = Color {
          r: 0xff,
          g: 0xff,
          b: 0xff
        };

        for x in 0..SCREEN_WIDTH {
          self.set_pixel(x as usize, y as usize, color);
        }
      },
      DisplayMode::Mode1 => self.render_normal_line(y, vram),
      DisplayMode::Mode2 => {
        for x in 0..SCREEN_WIDTH {
          let index = 2 * (y as usize * SCREEN_WIDTH as usize + x as usize);
          let bank = vram.get_lcdc_bank(self.dispcnt.vram_block);

          let color = bank[index] as u16 | (bank[(index + 1) as usize] as u16) << 8;

          let color = Color::from(color);

          self.set_pixel(x as usize, y as usize, color);
        }
      }
      DisplayMode::Mode3 => todo!()
    }
  }

  pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
    let i: usize = 3 * (x + y * SCREEN_WIDTH as usize);

    self.pixels[i] = color.r;
    self.pixels[i + 1] = color.g;
    self.pixels[i + 2] = color.b;

  }

  pub fn read_register(&self, address: u32) -> u16 {
    match address & 0xff {
      0x08 => self.bgcnt[0].bits(),
      0x0a => self.bgcnt[1].bits(),
      0x0c => self.bgcnt[2].bits(),
      0x0e => self.bgcnt[3].bits(),
      0x40 => self.winh[0].x1,
      0x42 => self.winh[1].x1,
      0x44 => self.winv[0].y1,
      0x46 => self.winv[1].y1,
      0x48 => self.winin.bits(),
      0x4a => self.winout.bits(),
      0x4c => 0, // TODO, see below
      0x50 => self.bldcnt.value,
      _ => panic!("invalid address given to engine read register method")
    }
  }

  pub fn write_register(&mut self, address: u32, value: u16, mask: Option<u16>) {
    let mut value = 0;

    if let Some(mask) = mask {
      value = self.read_register(address) & mask;
    }

    value |= value;

    let bg_props = &mut self.bg_props;

    macro_rules! write_bg_reference_point {
      (low $coordinate:ident $internal:ident $i:expr) => {{
        let existing = bg_props[$i].$coordinate as u32;

        let new_value = ((existing & 0xffff0000) + (value as u32)) as i32;

        bg_props[$i].$coordinate = new_value;
        bg_props[$i].$internal = new_value;
      }};
      (high $coordinate:ident $internal:ident $i:expr) => {{
        let existing = bg_props[$i].$coordinate;

        let new_value = existing & 0xffff | (((value & 0xfff) as i32) << 20) >> 4;

        bg_props[$i].$coordinate = new_value;
        bg_props[$i].$internal = new_value;
      }}
    }

    match address & 0xff {
      0x08 => self.bgcnt[0] = BgControlRegister::from_bits_retain(value),
      0x0a => self.bgcnt[1] = BgControlRegister::from_bits_retain(value),
      0x0c => self.bgcnt[2] = BgControlRegister::from_bits_retain(value),
      0x0e => self.bgcnt[3] = BgControlRegister::from_bits_retain(value),
      0x10 => self.bgxofs[0] = value & 0b111111111,
      0x12 => self.bgyofs[0] = value & 0b111111111,
      0x14 => self.bgxofs[1] = value & 0b111111111,
      0x16 => self.bgyofs[1] = value & 0b111111111,
      0x18 => self.bgxofs[2] = value & 0b111111111,
      0x1a => self.bgyofs[2] = value & 0b111111111,
      0x1c => self.bgxofs[3] = value & 0b111111111,
      0x1e => self.bgyofs[3] = value & 0b111111111,
      0x20 => self.bg_props[0].dx = value as i16,
      0x22 => self.bg_props[0].dmx = value as i16,
      0x24 => self.bg_props[0].dy = value as i16,
      0x26 => self.bg_props[0].dmy = value as i16,
      0x28 => write_bg_reference_point!(low x internal_x 0),
      0x2a => write_bg_reference_point!(high x internal_x 0),
      0x2c => write_bg_reference_point!(low y internal_y 0),
      0x2e => write_bg_reference_point!(high y internal_y 0),
      0x30 => self.bg_props[1].dx = value as i16,
      0x32 => self.bg_props[1].dmx = value as i16,
      0x34 => self.bg_props[1].dy = value as i16,
      0x36 => self.bg_props[1].dmy = value as i16,
      0x38 => write_bg_reference_point!(low x internal_x 1),
      0x3a => write_bg_reference_point!(high x internal_x 1),
      0x3c => write_bg_reference_point!(low y internal_y 1),
      0x3e => write_bg_reference_point!(high y internal_y 1),
      0x40 => self.winh[0].write(value),
      0x42 => self.winh[1].write(value),
      0x44 => self.winv[0].write(value),
      0x46 => self.winv[1].write(value),
      0x48 => self.winin = WindowInRegister::from_bits_retain(value),
      0x4a => self.winout = WindowOutRegister::from_bits_retain(value),
      0x4c => (), // TODO (but probably not lmao, mosaic is pointless)
      0x50 => self.bldcnt.write(value),
      0x52 => self.bldalpha.write(value),
      0x54 => self.bldy.write(value),
      _ => panic!("invalid address given to engine write register method")
    }
  }

  fn get_pixel_index_bpp8(&self, address: u32, tile_x: u16, tile_y: u16, x_flip: bool, y_flip: bool, vram: &VRam) -> u8 {
    let tile_x = if x_flip { 7 - tile_x } else { tile_x };
    let tile_y = if y_flip { 7 - tile_y } else { tile_y };

    if !IS_ENGINE_B {
      vram.read_engine_a_bg(address + tile_x as u32 + (tile_y as u32) * 8)
    } else {
      0
    }
    // self.vram[(address + tile_x as u32 + (tile_y as u32) * 8) as usize]
  }

  fn get_pixel_index_bpp4(&self, address: u32, tile_x: u16, tile_y: u16, x_flip: bool, y_flip: bool, vram: &VRam) -> u8 {
    let tile_x = if x_flip { 7 - tile_x } else { tile_x };
    let tile_y = if y_flip { 7 - tile_y } else { tile_y };

    let address = address + (tile_x / 2) as u32 + (tile_y as u32) * 4;

    let byte = vram.read_engine_a_bg(address);

    if tile_x & 0b1 == 1 {
      byte >> 4
    } else {
      byte & 0xf
    }
  }
}