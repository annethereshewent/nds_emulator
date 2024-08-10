
#[derive(Debug)]
pub enum TextureFormat {
  None,
  A3I5Transluscent,
  Color4,
  Color16,
  Color256,
  Color4x4,
  A5I3Transluscent,
  Direct
}


#[derive(Copy, Clone, PartialEq)]
pub enum TransformationMode {
  None = 0,
  TexCoord = 1,
  Normal = 2,
  Vertex = 3
}

bitflags! {
  #[derive(Copy, Clone, Debug)]
  pub struct TextureParams: u32 {
    const REPEAT_S = 1 << 16;
    const REPEAT_T = 1 << 17;
    const FLIP_S = 1 << 18;
    const FLIP_T = 1 << 19;
    const COLOR0_TRANSPARENT = 1 << 29;
  }
}

impl TextureParams {
  pub fn vram_offset(&self) -> u32 {
    (self.bits() & 0xffff) << 3
  }

  pub fn texture_s_size(&self) -> u32 {
    8 << (self.bits() >> 20 & 0x7)
  }

  pub fn texture_t_size(&self) -> u32 {
    8 << (self.bits() >> 23 & 0x7)
  }

  pub fn size_s_shift(&self) -> u32 {
    3 + (self.bits() >> 20 & 0x7)
  }

  pub fn size_t_shift(&self) -> u32 {
    3 + (self.bits() >> 23 & 0x7)
  }

  pub fn texture_format(&self) -> TextureFormat {
    match self.bits() >> 26 & 0x7 {
      0 => TextureFormat::None,
      1 => TextureFormat::A3I5Transluscent,
      2 => TextureFormat::Color4,
      3 => TextureFormat::Color16,
      4 => TextureFormat::Color256,
      5 => TextureFormat::Color4x4,
      6 => TextureFormat::A5I3Transluscent,
      7 => TextureFormat::Direct,
      _ => unreachable!()
    }
  }

  pub fn transformation_mode(&self) -> TransformationMode {
    match self.bits() >> 30 & 0x3 {
      0 => TransformationMode::None,
      1 => TransformationMode::TexCoord,
      2 => TransformationMode::Normal,
      3 => TransformationMode::Vertex,
      _ => unreachable!()
    }
  }
}