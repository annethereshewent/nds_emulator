use std::cmp;

use crate::gpu::{color::Color, engine_3d::texture_params::TextureParams, vram::VRam, SCREEN_WIDTH};

use super::{polygon::Polygon, polygon_attributes::PolygonMode, texture_params::TextureFormat, vertex::Vertex, Engine3d, Pixel3d};

#[derive(Debug)]
pub struct TextureDeltas {
  current: usize,
  start: f32,
  num_steps: f32,
  w_start: f32,
  w_end: f32,
  diff: f32,
  dw: f32
}

impl TextureDeltas {
  pub fn new(start: f32, w_start: f32, w_end: f32, diff: f32, num_steps: f32) -> Self {
    Self {
      start,
      current: 0,
      w_start,
      w_end,
      diff,
      num_steps,
      dw: (w_end - w_start) / num_steps
    }
  }

  pub fn next(&mut self) -> f32 {
    let current = self.current as f32;
    let factor = (current * self.w_start) / (((self.num_steps - current) * self.w_end) + (current * self.w_start));
    self.current += 1;
    self.start + factor * self.diff
  }

  pub fn get_texture_deltas(start: Option<Vertex>, end: Option<Vertex>, is_u: bool) -> Self {
    let start = start.unwrap();
    let end = end.unwrap();

    let (start_fp, end_fp) = if is_u {
      (start.texcoord.u as f32, end.texcoord.u as f32)
    } else {
      (start.texcoord.v as f32, end.texcoord.v as f32)
    };

    let deltas = TextureDeltas::new(
      start_fp,
      start.normalized_w as f32,
      end.normalized_w as f32,
      end_fp - start_fp,
      (end.screen_y - start.screen_y) as f32
    );

    deltas
  }
}

impl Engine3d {
  pub fn cross_product(ax: i32, ay: i32, bx: i32, by: i32, cx: i32, cy: i32) -> i32 {
    (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
  }

  pub fn start_rendering(&mut self, vram: &VRam) {
    if self.polygons_ready {
      for polygon in self.polygon_buffer.drain(..) {
        let vertices = &mut self.vertices_buffer[polygon.start..polygon.end];

        if vertices.len() == 3 {
          Self::rasterize_triangle(&polygon, vertices, vram, &mut self.frame_buffer);
        } else {
          // break up into multiple triangles and then render the triangles
          let mut i = 0;
          vertices.sort_by(|a, b| {
            if a.screen_y != b.screen_y {
              a.screen_y.cmp(&b.screen_y)
            } else {
              a.screen_x.cmp(&b.screen_x)
            }
          });
          while i + 2 < vertices.len() {
            let mut cloned = [Vertex::new(); 3];

            cloned.clone_from_slice(&vertices[i..i + 3]);

            Self::rasterize_triangle(&polygon, &mut cloned, vram, &mut self.frame_buffer);

            i += 1;
          }
        }
      }

      self.vertices_buffer.clear();
      self.polygons_ready = false;
      self.gxstat.geometry_engine_busy = false;
    }
  }

  fn get_palette_color(polygon: &Polygon, palette_base: u32, palette_index: u32, vram: &VRam, alpha: Option<u8>) -> (Option<Color>, Option<u8>) {
    let address = palette_base + 2 * palette_index;

    let color_raw = vram.read_texture_palette(address) as u16 | (vram.read_texture_palette(address + 1) as u16) << 8;

    if palette_index == 0 && polygon.tex_params.contains(TextureParams::COLOR0_TRANSPARENT) && alpha.is_none() {
      (Some(Color::from(color_raw)), Some(0))
    } else {
      (Some(Color::from(color_raw)), alpha)
    }
  }

  fn rasterize_triangle(polygon: &Polygon, vertices: &mut [Vertex], vram: &VRam, frame_buffer: &mut [Pixel3d]) {
    vertices.sort_by(|a, b| a.screen_y.cmp(&b.screen_y));

    let cross_product = Self::cross_product(
    vertices[0].screen_x as i32,
    vertices[0].screen_y as i32,
    vertices[1].screen_x as i32,
    vertices[1].screen_y as i32,
    vertices[2].screen_x as i32,
    vertices[2].screen_y as i32
    );

    if cross_product == 0 {
      return;
    }

    let p02_is_left = cross_product > 0;

    let min_y = cmp::min(vertices[0].screen_y, cmp::min(vertices[1].screen_y, vertices[2].screen_y));
    let max_y = cmp::max(vertices[0].screen_y, cmp::max(vertices[1].screen_y, vertices[2].screen_y));

    let min_x = cmp::min(vertices[0].screen_x, cmp::min(vertices[1].screen_x, vertices[2].screen_x));

    let mut left_start: Option<Vertex> = None;
    let mut left_end: Option<Vertex> = None;

    let mut right_end: Option<Vertex> = None;
    let mut right_start: Option<Vertex> = None;


    let p01_slope = if vertices[0].screen_y != vertices[1].screen_y {
      let slope = (vertices[1].screen_x as i32 - vertices[0].screen_x as i32) as f32 / (vertices[1].screen_y as i32 - vertices[0].screen_y as i32) as f32;
      if p02_is_left {
        right_start = Some(vertices[0]);
        right_end =  Some(vertices[1]);
      } else {
        left_start = Some(vertices[0]);
        left_end = Some(vertices[1]);
      }
      Some(slope)
    } else {
      None
    };

    let p12_slope = if vertices[1].screen_y != vertices[2].screen_y {
      let slope = (vertices[2].screen_x as i32 - vertices[1].screen_x as i32) as f32 / (vertices[2].screen_y as i32 - vertices[1].screen_y as i32) as f32;

      if p02_is_left {
        right_start = Some(vertices[1]);
        right_end =  Some(vertices[2]);
      } else {
        left_start = Some(vertices[1]);
        left_end = Some(vertices[2]);
      }
      Some(slope)
    } else {
      None
    };

    let p02_slope = if vertices[0].screen_y != vertices[2].screen_y {
      let slope = (vertices[2].screen_x as i32 - vertices[0].screen_x as i32) as f32 / (vertices[2].screen_y as i32 - vertices[0].screen_y as i32) as f32;

      if p02_is_left {
        left_start = Some(vertices[0]);
        left_end =  Some(vertices[2]);
      } else {
        right_start = Some(vertices[0]);
        right_end = Some(vertices[2]);
      }

      Some(slope)
    } else {
      None
    };

    let mut left_vertical_u = TextureDeltas::get_texture_deltas(left_start, left_end, true);
    let mut right_vertical_u = TextureDeltas::get_texture_deltas(right_start, right_end, true);

    let mut left_vertical_v = TextureDeltas::get_texture_deltas(left_start, left_end, false);
    let mut right_vertical_v = TextureDeltas::get_texture_deltas(right_start, right_end, false);

    let mut y = min_y;
    let mut x = min_x;

    let mut w_start = left_start.unwrap().normalized_w as f32;

    let mut w_end = right_start.unwrap().normalized_w as f32;
    while y < max_y {
      let left_u = left_vertical_u.next();
      let right_u = right_vertical_u.next();

      let left_v = left_vertical_v.next();
      let right_v = right_vertical_v.next();

      let (boundary1, boundary2) = Self::get_triangle_boundaries(vertices, p01_slope, p02_slope, p12_slope, y as i32);

      x = boundary1 as u32;

      let left_start = left_start.unwrap();
      let right_start = right_start.unwrap();

      // let rel_y_left = y as i16 - left_start.screen_y as i16;
      // let rel_y_right = y as i16 - right_start.screen_y as i16;

      // let w_start = ((left_vertical_u.dw * rel_y_left as f32) as i16) + left_start.normalized_w;
      // let w_end = ((right_vertical_u.dw * rel_y_right as f32) as i16) + right_start.normalized_w;

      w_start += left_vertical_u.dw;
      w_end += right_vertical_u.dw;

      let mut u_d = TextureDeltas::new(
        left_u,
        w_start as f32,
        w_end as f32,
        right_u - left_u,
        (boundary2 - boundary1) as f32
      );

      let mut v_d = TextureDeltas::new(
        left_v,
        w_start as f32,
        w_end as f32,
        right_v - left_v,
        (boundary2 - boundary1) as f32
      );

      while x < boundary2 as u32 {
        let curr_u = (u_d.next() as u32 >> 4).clamp(0, polygon.tex_params.texture_s_size() - 1);
        let curr_v = (v_d.next() as u32 >> 4).clamp(0, polygon.tex_params.texture_t_size() - 1);

        // render the pixel!
        let pixel = &mut frame_buffer[(x + y * SCREEN_WIDTH as u32) as usize];

        let (texel_color, alpha) = Self::get_texel_color(polygon, curr_u, curr_v, vram);

        if let Some(texel_color) = texel_color {
          pixel.color = if alpha.is_some() && alpha.unwrap() == 0 {
            None
          } else {
            // check to see if color is blended
            match polygon.attributes.polygon_mode() {
              PolygonMode::Decal => {
                todo!("decal mode not implemented");
              }
              PolygonMode::Modulation => {
                Self::modulation_blend(texel_color, vertices[0].color, alpha)
              }
              PolygonMode::Shadow => {
                todo!("shadow mode not implemented");
              }
              PolygonMode::Toon => {
                todo!("toon mode not implemented");
              }
            }
          }
        } else {
          pixel.color = Some(vertices[0].color);
        }
        x += 1;
      }
      y += 1;
    }
  }

  fn modulation_blend(texel: Color, pixel: Color, alpha: Option<u8>) -> Option<Color> {
    // ((val1 + 1) * (val2 + 1) - 1) / 64;
    let modulation_fn = |component1, component2| ((component1 + 1) * (component2 + 1) - 1) / 64;

    let r = modulation_fn(texel.r as u16, pixel.r as u16) as u8;
    let g = modulation_fn(texel.g as u16, pixel.g as u16) as u8;
    let b = modulation_fn(texel.b as u16, pixel.b as u16) as u8;

    Some(Color {
      r,
      g,
      b,
      alpha
    })
  }

  fn get_texel_color(polygon: &Polygon, curr_u: u32, curr_v: u32, vram: &VRam) -> (Option<Color>, Option<u8>) {
    let texel = curr_u + curr_v * polygon.tex_params.texture_s_size();
    let vram_offset = polygon.tex_params.vram_offset();

    let address = vram_offset + texel;

    let palette_base = polygon.palette_base;

    match polygon.tex_params.texture_format() {
      TextureFormat::None => {
        (None, None)
      },
      TextureFormat::A315Transluscent => {
        let byte = vram.read_texture(address);

        let palette_index = byte & 0x1f;
        let alpha = (byte >> 5) & 0x7;

        Self::get_palette_color(polygon, palette_base as u32, palette_index as u32, vram, Some(alpha * 4 + alpha / 2))
      }
      TextureFormat::A513Transluscent => {
        let byte = vram.read_texture(address);

        let palette_index = byte & 0x7;

        let alpha = (byte >> 3) & 0x1f;

        Self::get_palette_color(polygon, palette_base as u32, palette_index as u32, vram, Some(alpha))
      }
      TextureFormat::Color16 => {
        let real_address = vram_offset + texel / 2;

        let byte = vram.read_texture(real_address);

        let palette_index = if texel & 0b1 == 0 {
          byte & 0xf
        } else {
          (byte >> 4) & 0xf
        };

        Self::get_palette_color(polygon, palette_base as u32, palette_index as u32, vram, None)
      }
      TextureFormat::Color256 => {
        let palette_index = vram.read_texture(address);

        Self::get_palette_color(polygon, palette_base as u32, palette_index as u32, vram, None)
      }
      TextureFormat::Color4x4 => {
        let blocks_per_row = polygon.tex_params.texture_s_size() / 4;

        let block_address = curr_u / 4 + blocks_per_row * curr_v / 4;

        let base_address = vram_offset + 4 * block_address;

        let mut texel_value = vram.read_texture(address);

        texel_value = match curr_u & 0x3 {
          0 => texel_value & 0x3,
          1 => texel_value >> 2 & 0x3,
          2 => texel_value >> 4 & 0x3,
          3 => texel_value >> 6 & 0x3,
          _ => unreachable!()
        };

        let slot1_address = base_address / 2 + if base_address > 128 * 0x400 {
          0x1000
        } else {
          0
        };

        let extra_palette_info = vram.read_texture(slot1_address) as u16 | (vram.read_texture(slot1_address + 1) as u16) << 8;

        let palette_offset = palette_base as u32 + ((extra_palette_info & 0x1fff) * 4) as u32;

        let mode = (extra_palette_info >> 14) & 0x3;

        match (texel_value, mode) {
          (0, _) => {
            // color 0
            let palette_index = vram.read_texture_palette(palette_offset);

            Self::get_palette_color(polygon, palette_offset, palette_index as u32, vram, None)
          }
          (1, _) => {
            // color 1
            let palette_index = vram.read_texture_palette(palette_offset + 2);

            Self::get_palette_color(polygon, palette_offset, palette_index as u32, vram, None)
          },
          (2, 0) | (2, 2) => {
            // color 2
            let palette_index = vram.read_texture_palette(palette_offset + 2 * 2);

            Self::get_palette_color(polygon, palette_offset, palette_index as u32, vram, None)
          }
          (2, 1) => {
            // (color0 + color1) / 2
            let palette0_index = vram.read_texture_palette(palette_offset);
            let palette1_index = vram.read_texture_palette(palette_offset + 2);

            let (color0, alpha1) = Self::get_palette_color(polygon, palette_offset, palette0_index as u32, vram, None);
            let (color1, alpha2) = Self::get_palette_color(polygon, palette_offset, palette1_index as u32, vram, None);

            let blended_color = color0.unwrap().blend_half(color1.unwrap());

            (Some(blended_color), alpha1)
          }
          (2, 3) => {
            // (color0 * 5 + color1 * 3) / 8
            let palette0_index = vram.read_texture_palette(palette_offset);
            let palette1_index = vram.read_texture_palette(palette_offset + 2);

            let (color0, alpha1) = Self::get_palette_color(polygon, palette_offset, palette0_index as u32, vram, None);
            let (color1,_) = Self::get_palette_color(polygon, palette_offset, palette1_index as u32, vram, None);

            let blended_color = color0.unwrap().blend_texture(color1.unwrap());

            (Some(blended_color), alpha1)
          }
          (3, 0)| (3, 1) => {
            // transparent
            (Some(Color { r: 0, g: 0, b: 0, alpha: Some(0) }), Some(0))
          }
          (3, 2) => {
            // color 3
            let palette_index = vram.read_texture_palette(palette_offset + 2 * 3);

            Self::get_palette_color(polygon, palette_offset, palette_index as u32, vram, None)
          }
          (3, 3) => {
            // (color0 * 3 + color1 * 5) / 8
            let palette0_index = vram.read_texture_palette(palette_offset);
            let palette1_index = vram.read_texture_palette(palette_offset + 2);

            let (color0, alpha1) = Self::get_palette_color(polygon, palette_offset, palette0_index as u32, vram, None);
            let (color1, _) = Self::get_palette_color(polygon, palette_offset, palette1_index as u32, vram, None);

            let blended_color = color1.unwrap().blend_texture(color0.unwrap());

            (Some(blended_color), alpha1)
          }
          _ => panic!("invalid options given for texel value and mode: {texel_value} {mode}")
        }
      }
      TextureFormat::Color4 => {
        let mut palette_index = vram.read_texture(vram_offset + texel / 4);

        palette_index = match texel & 0x3 {
          0 => palette_index & 0x3,
          1 => (palette_index >> 2) & 0x3,
          2 => (palette_index >> 4) & 0x3,
          3 => (palette_index >> 6) & 0x3,
          _ => unreachable!()
        };
        Self::get_palette_color(polygon, palette_base as u32, palette_index as u32, vram, None)
      }
      TextureFormat::Direct => {
        let address = vram_offset + 2 * texel;
        let color_raw = vram.read_texture(address) as u16 | (vram.read_texture(address + 1) as u16) << 8;

        let alpha = if color_raw & 0x8000 == 0 { 0 } else { 0x1f };

        (Some(Color::from(color_raw)), Some(alpha))
      }
    }
  }

  fn get_triangle_boundaries(vertices: &[Vertex], p01_slope: Option<f32>, p02_slope: Option<f32>, p12_slope: Option<f32>, y: i32) -> (i32, i32) {
    let mut boundary2 = 0;

    // three cases to consider: p02 is always horizontal because vertices are sorted
    // by y coordinate, so either p01 slope is horizontal, p12 is, or neither are.
    if p01_slope.is_none() {
      let p12_slope = p12_slope.unwrap();

      let rel_y = y - vertices[1].screen_y as i32;

      boundary2 = ((p12_slope * rel_y as f32) + vertices[1].screen_x as f32) as i32;

    } else if p12_slope.is_none() {
      let p01_slope = p01_slope.unwrap();

      let rel_y = y as i32 - vertices[0].screen_y as i32;

      boundary2 = ((p01_slope * rel_y as f32) + vertices[0].screen_x as f32) as i32;
    } else {
      // neither slope is horizontal, determine which slope to use based on y coordinate.
      // if y coordinate is less than vertex 1's y coordinate, then use p01 slope
      // otherwise, boundary must be in p12 slope
      if y < vertices[1].screen_y as i32 {
        let p01_slope = p01_slope.unwrap();

        let rel_y = y - vertices[0].screen_y as i32;

        boundary2 = ((p01_slope * rel_y as f32) + vertices[0].screen_x as f32) as i32;
      } else {
        let p12_slope = p12_slope.unwrap();

        let rel_y = y - vertices[1].screen_y as i32;

        boundary2 = ((p12_slope * rel_y as f32) + vertices[1].screen_x as f32) as i32;
      }
    }

    let p02_slope = p02_slope.unwrap();

    let rel_y = y - vertices[0].screen_y as i32;

    let boundary1 = ((p02_slope * rel_y as f32) + vertices[0].screen_x as f32) as i32;

    if boundary2 > boundary1 {
      (boundary1, boundary2)
    } else {
      (boundary2, boundary1)
    }
  }
}