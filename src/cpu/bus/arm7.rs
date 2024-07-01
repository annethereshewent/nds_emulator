use super::Bus;

impl Bus {
  pub fn arm7_mem_read_32(&mut self, address: u32) -> u32 {
    self.arm7_mem_read_16(address) as u32 | ((self.arm7_mem_read_16(address + 2) as u32) << 16)
  }

  pub fn arm7_mem_read_16(&mut self, address: u32) -> u16 {
    match address {
      0x400_0000..=0x4ff_ffff => self.arm7_io_read_16(address),
      _ => self.arm7_mem_read_8(address) as u16 | ((self.arm7_mem_read_8(address + 1) as u16) << 8)
    }
  }

  pub fn arm7_mem_read_8(&mut self, address: u32) -> u8 {
    let bios_len = self.arm7.bios7.len() as u32;

    if (0..bios_len).contains(&address) {
      return self.arm7.bios7[address as usize];
    }

    match address {
      0x400_0000..=0x4ff_ffff => self.arm7_io_read_8(address),
      0x700_0000..=0x7ff_ffff => 0,
      0x800_0000..=0xdff_ffff => {
        0
      }
      _ => {
        panic!("reading from unsupported address: {:X}", address);
      }
    }
  }

  fn arm7_io_read_16(&mut self, address: u32) -> u16 {
    let address = if address & 0xfffe == 0x8000 {
      0x400_0800
    } else {
      address
    };

    match address {
      0x400_0300 => self.arm7.postflg as u16,
      _ => {
        panic!("io register not implemented: {:X}", address);
      }
    }
  }

  fn arm7_io_read_8(&mut self, address: u32) -> u8 {
    let val = self.arm7_io_read_16(address & !(0b1));

    if address & 0b1 == 1 {
      (val >> 8) as u8
    } else {
      (val & 0xff) as u8
    }
  }

  pub fn arm7_mem_write_32(&mut self, address: u32, val: u32) {
    let upper = (val >> 16) as u16;
    let lower = (val & 0xffff) as u16;

    self.arm7_mem_write_16(address, lower);
    self.arm7_mem_write_16(address + 2, upper);
  }

  pub fn arm7_mem_write_16(&mut self, address: u32, val: u16) {
    let upper = (val >> 8) as u8;
    let lower = (val & 0xff) as u8;

    match address {
      0x400_0000..=0x4ff_ffff => self.arm7_io_write_16(address, val),
      _ => {
        self.arm7_mem_write_8(address, lower);
        self.arm7_mem_write_8(address + 1, upper);
      }
    }
  }

  pub fn arm7_mem_write_8(&mut self, address: u32, val: u8) {
    match address {
      0x400_0000..=0x4ff_ffff => self.arm7_io_write_8(address, val),
      0x500_0000..=0x5ff_ffff => self.arm7_mem_write_16(address & 0x3fe, (val as u16) * 0x101),
      _ => {
        panic!("writing to unsupported address: {:X}", address);
      }
    }
  }

  pub fn arm7_io_write_16(&mut self, address: u32, value: u16) {
    let address = if address & 0xfffe == 0x8000 {
      0x400_0800
    } else {
      address
    };

    match address {
      0x400_0006 => (),
      _ => {
        panic!("io register not implemented: {:X}", address)
      }
    }
  }

  pub fn arm7_io_write_8(&mut self, address: u32, value: u8) {
    let address = if address & 0xffff == 0x8000 {
      0x400_0800
    } else {
      address
    };

    // println!("im being called with address {:X}", address);

    match address {
      _ => {
        let mut temp = self.arm7_mem_read_16(address & !(0b1));

        temp = if address & 0b1 == 1 {
          (temp & 0xff) | (value as u16) << 8
        } else {
          (temp & 0xff00) | value as u16
        };

        self.arm7_mem_write_16(address & !(0b1), temp);
      }
    }

    // todo: implement sound
  }
}