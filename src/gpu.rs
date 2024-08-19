use std::{
  hint, sync::{
    atomic::{AtomicBool, AtomicU16, Ordering}, Arc, Mutex, MutexGuard
  }, thread::{
    self, sleep, JoinHandle
  }, time::{
    Duration,
    SystemTime,
    UNIX_EPOCH
  }
};

use engine_2d::{
  renderer2d::Renderer2d,
  Engine2d,
  ObjectPixel
};
use engine_3d::{
  Engine3d,
  Pixel3d
};

use crate::number::Number;
use registers::{
  display_capture_control_register::{
    CaptureSource,
    DisplayCaptureControlRegister,
    ScreenSourceA,
    ScreenSourceB
  },
  display_control_register::DisplayMode,
  display_status_register::{
    DispStatFlags,
    DisplayStatusRegister
  },
  power_control_register1::PowerControlRegister1,
  power_control_register2::PowerControlRegister2,
  vram_control_register::VramControlRegister,
};
use rendering_data::RenderingData;
use rendering_data3d::RenderingData3d;
use vram::{Bank, VRam};

use crate::{
  cpu::{
    dma::{
      dma_channel::registers::dma_control_register::DmaTiming,
      dma_channels::DmaChannels
    }, registers::{
      interrupt_request_register::InterruptRequestRegister,
      mosaic_register::MosaicRegister
    }
  },
  scheduler::{
    EventType,
    Scheduler
  }
};

pub mod registers;
pub mod engine_2d;
pub mod engine_3d;
pub mod vram;
pub mod color;
pub mod rendering_data;
pub mod rendering_data3d;

const NUM_LINES: u16 = 263;

pub const SCREEN_HEIGHT: u16 = 192;
pub const SCREEN_WIDTH: u16 = 256;

pub const HBLANK_CYCLES: usize = 1606;
pub const HDRAW_CYCLES: usize = 524;

pub const FPS_INTERVAL: u128 = 1000 / 60;

const BANK_A: u32 = Bank::BankA as u32;
const BANK_B: u32 = Bank::BankB as u32;
const BANK_C: u32 = Bank::BankC as u32;
const BANK_D: u32 = Bank::BankD as u32;
const BANK_E: u32 = Bank::BankE as u32;
const BANK_F: u32 = Bank::BankF as u32;
const BANK_G: u32 = Bank::BankG as u32;
const BANK_H: u32 = Bank::BankH as u32;
const BANK_I: u32 = Bank::BankI as u32;

#[derive(Copy, Clone)]
pub struct BgProps {
  pub x: i32,
  pub y: i32,
  pub dx: i16,
  pub dmx: i16,
  pub dy: i16,
  pub dmy: i16,
  pub internal_x: i32,
  pub internal_y: i32
}

impl BgProps {
  pub fn new() -> Self {
    Self {
      x: 0,
      y: 0,
      dx: 0,
      dmx: 0,
      dy: 0,
      dmy: 0,
      internal_x: 0,
      internal_y: 0
    }
  }
}

pub struct ThreadData {
  pub vram: Mutex<VRam>,
  pub vcount: AtomicU16,
  pub frame_buffer: Mutex<[Pixel3d; SCREEN_HEIGHT as usize * SCREEN_WIDTH as usize]>,
  pub dispcapcnt: Mutex<DisplayCaptureControlRegister>,
  pub is_capturing: AtomicBool,
  pub finished_line: AtomicBool,
  pub powcnt1: Mutex<PowerControlRegister1>,
  pub rendering_data: [Mutex<RenderingData>; 2],
  pub rendering_data3d: Mutex<RenderingData3d>
}

pub struct GPU {
  pub engine_a: Engine2d<false>,
  pub engine_b: Engine2d<true>,
  pub engine3d: Engine3d,
  pub powcnt2: PowerControlRegister2,
  pub powcnt1: PowerControlRegister1,
  pub vramcnt: [VramControlRegister; 9],
  pub dispstat: [DisplayStatusRegister; 2],
  pub frame_finished: bool,
  pub mosaic: MosaicRegister,
  previous_time: u128,
  rendering2d_thread: Option<JoinHandle<()>>,
  pub thread_data: Arc<ThreadData>,
  pub dispcapcnt: DisplayCaptureControlRegister,
  pub vram: VRam
}

impl GPU {
  pub fn new(scheduler: &mut Scheduler) -> Self {
    let mut vramcnt: Vec<VramControlRegister> = Vec::new();

    for i in 0..9 {
      vramcnt.push(VramControlRegister::new(i));
    }

    let thread_data = Arc::new(ThreadData {
      vcount: AtomicU16::new(0),
      powcnt1: Mutex::new(PowerControlRegister1::from_bits_retain(0)),
      is_capturing: AtomicBool::new(false),
      dispcapcnt: Mutex::new(DisplayCaptureControlRegister::new()),
      vram: Mutex::new(VRam::new()),
      finished_line: AtomicBool::new(false),
      rendering_data: [
        Mutex::new(RenderingData::new()),
        Mutex::new(RenderingData::new())
      ],
      frame_buffer: Mutex::new([Pixel3d::new(); SCREEN_HEIGHT as usize * SCREEN_WIDTH as usize]),
      rendering_data3d: Mutex::new(RenderingData3d::new()),
    });

    let mut gpu = Self {
      engine_a: Engine2d::new(),
      engine_b: Engine2d::new(),
      engine3d: Engine3d::new(),
      powcnt2: PowerControlRegister2::from_bits_retain(0),
      powcnt1: PowerControlRegister1::from_bits_retain(0),
      vramcnt: vramcnt.try_into().unwrap(),
      dispstat: [DisplayStatusRegister::new(), DisplayStatusRegister::new()],
      frame_finished: false,
      mosaic: MosaicRegister::new(),
      previous_time: 0,
      rendering2d_thread: None,
      thread_data: thread_data.clone(),
      dispcapcnt: DisplayCaptureControlRegister::new(),
      vram: VRam::new()
    };

    let mut renderer2d = Renderer2d {
      thread_data: thread_data.clone()
    };

    gpu.rendering2d_thread = Some(
      thread::spawn(move || {
        loop {
          if !thread_data.finished_line.load(Ordering::Relaxed) {
            hint::spin_loop();
            continue;
          }

          let vcount = thread_data.vcount.load(Ordering::Acquire);

          if vcount >= SCREEN_HEIGHT {
            thread::park();
            continue;
          }

          let powcnt1 = thread_data.powcnt1.lock().unwrap();
          let mut vram = thread_data.vram.lock().unwrap();
          let frame_buffer = thread_data.frame_buffer.lock().unwrap();
          let mut dispcapcnt = thread_data.dispcapcnt.lock().unwrap();
          let rendering_data_a = thread_data.rendering_data[0].lock().unwrap();

          if powcnt1.contains(PowerControlRegister1::ENGINE_A_ENABLE) {
            renderer2d.render_line(vcount, &mut vram, &frame_buffer, false);

            // capture image if needed
            if thread_data.is_capturing.load(Ordering::Relaxed) && vcount < dispcapcnt.get_capture_height() {
              dispcapcnt.capture_enable = false;
              Self::start_capture_image(&mut dispcapcnt, &rendering_data_a, &frame_buffer, vcount, &mut vram);
            }
          }
          if powcnt1.contains(PowerControlRegister1::ENGINE_B_ENABLE) {
            renderer2d.render_line(vcount, &mut vram, &frame_buffer, true);
          }

          thread_data.finished_line.store(false, Ordering::Release);
          drop(powcnt1);
          drop(vram);
          drop(frame_buffer);
          drop(rendering_data_a);
          drop(dispcapcnt);
        }
      })
    );

    scheduler.schedule(EventType::HBlank, HBLANK_CYCLES);

    gpu
  }

  pub fn cap_fps(&mut self) {
    let current_time = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("an error occurred")
      .as_millis();

    if self.previous_time != 0 {
      let diff = current_time - self.previous_time;

      if diff < FPS_INTERVAL {
        sleep(Duration::from_millis((FPS_INTERVAL - diff) as u64));
      }
    }

    self.previous_time = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("an error occurred")
      .as_millis();
  }

  pub fn handle_hblank(
    &mut self,
    scheduler: &mut Scheduler,
    interrupt_requests: &mut [&mut InterruptRequestRegister],
    dma_channels: &mut [&mut DmaChannels],
    cycles_left: usize)
  {
    self.schedule_hdraw(scheduler, cycles_left);

    for dispstat in &mut self.dispstat {
      dispstat.flags.insert(DispStatFlags::HBLANK);
    }

    for dma in dma_channels {
      dma.notify_gpu_event(DmaTiming::Hblank);
    }

    Self::check_interrupts(&mut self.dispstat, DispStatFlags::HBLANK_IRQ_ENABLE, InterruptRequestRegister::HBLANK, interrupt_requests);
  }

  pub fn check_interrupts(dispstat: &mut [DisplayStatusRegister], dispstat_flag: DispStatFlags, interrupt_flag: InterruptRequestRegister, interrupt_requests: &mut [&mut InterruptRequestRegister]) {
    for i in 0..2 {
      let dispstat = &mut dispstat[i];
      let interrupt_request = &mut interrupt_requests[i];

      if dispstat.flags.contains(dispstat_flag) {
        interrupt_request.insert(interrupt_flag);
      }
    }
  }

  fn flush_frame_buffer(&mut self) {
    let mut frame_buffer = self.thread_data.frame_buffer.lock().unwrap();

    frame_buffer.copy_from_slice(&self.engine3d.frame_buffer);
  }

  fn flush_rendering_data(&mut self) {
    let data = &mut self.thread_data;
    let mut rendering_data_a = data.rendering_data[0].lock().unwrap();
    let mut rendering_data_b = data.rendering_data[1].lock().unwrap();
    let mut vram = data.vram.lock().unwrap();

    if self.vram.updated.len() > 0 {
      *vram = self.vram.clone();
      self.vram.updated.clear();
    }

    self.engine_a.pixels = rendering_data_a.pixels;
    self.engine_b.pixels = rendering_data_b.pixels;
    self.engine_a.pixel_alphas = rendering_data_a.pixel_alphas;
    self.engine_b.pixel_alphas = rendering_data_b.pixel_alphas;

    let mut powcnt1 = data.powcnt1.lock().unwrap();

    *powcnt1 = self.powcnt1;

    macro_rules! set_rendering_data {
      ($engine:ident) => {{
        RenderingData {
          dispcnt: self.$engine.dispcnt,
          bgcnt: self.$engine.bgcnt,
          oam: self.$engine.oam,
          pixels: self.$engine.pixels,
          bgxofs: self.$engine.bgxofs,
          bgyofs: self.$engine.bgyofs,
          bg_props: self.$engine.bg_props,
          winh: self.$engine.winh,
          winv: self.$engine.winv,
          winin: self.$engine.winin,
          winout: self.$engine.winout,
          bldcnt: self.$engine.bldcnt,
          bldalpha: self.$engine.bldalpha,
          bldy: self.$engine.bldy,
          bg_lines: [[None; SCREEN_WIDTH as usize]; 4],
          master_brightness: self.$engine.master_brightness,
          palette_ram: self.$engine.palette_ram,
          obj_lines: [ObjectPixel::new(); SCREEN_WIDTH as usize],
          pixel_alphas: self.$engine.pixel_alphas
        }
      }};
    }

    *rendering_data_a = set_rendering_data!(engine_a);
    *rendering_data_b = set_rendering_data!(engine_b);

  }

  pub fn start_next_line(
    &mut self, scheduler: &mut Scheduler,
    interrupt_requests: &mut [&mut InterruptRequestRegister],
    dma_channels: &mut [&mut DmaChannels],
    cycles_left: usize)
  {
    scheduler.schedule(EventType::HBlank, HBLANK_CYCLES - cycles_left);

    let mut vcount = self.thread_data.vcount.load(Ordering::Acquire);

    vcount += 1;

    if vcount > 0 && vcount <= SCREEN_HEIGHT {
      while self.thread_data.finished_line.load(Ordering::Acquire) {
        hint::spin_loop();
      }

      self.flush_rendering_data();
    }

    self.thread_data.finished_line.store(true, Ordering::Release);

    if vcount == NUM_LINES {
      vcount = 0;

      self.engine_a.on_end_vblank();
      self.engine_b.on_end_vblank();
    }

    if vcount == 0 {
      self.rendering2d_thread.as_ref().unwrap().thread().unpark();

      self.thread_data.is_capturing.store(
        self.thread_data.dispcapcnt.lock().unwrap().capture_enable,
        Ordering::Relaxed
      );

      for dispstat in &mut self.dispstat {
        dispstat.flags.remove(DispStatFlags::VBLANK);
      }
    } else if vcount == SCREEN_HEIGHT {
      if self.thread_data.is_capturing.load(Ordering::Relaxed) {
        self.thread_data.dispcapcnt.lock().unwrap().capture_enable = false;
      }
      for dispstat in &mut self.dispstat {
        dispstat.flags.insert(DispStatFlags::VBLANK);
      }

      for dma in &mut *dma_channels {
        dma.notify_gpu_event(DmaTiming::Vblank);
      }

      self.frame_finished = true;

      Self::check_interrupts(&mut self.dispstat, DispStatFlags::VBLANK_IRQ_ENABLE, InterruptRequestRegister::VBLANK, interrupt_requests);
    } else if vcount == NUM_LINES - 48 {
      // per martin korth, "Rendering starts 48 lines in advance (while still in the Vblank period)"
      let powcnt1 = self.thread_data.powcnt1.lock().unwrap();

      if powcnt1.contains(PowerControlRegister1::ENGINE_3D_ENABLE) {
        drop(powcnt1);
        self.engine3d.start_rendering(&self.vram);

        self.flush_frame_buffer();

        self.engine3d.execute_commands(&mut interrupt_requests[1]);

        if self.engine3d.should_run_dmas() {
          for dma in dma_channels {
            dma.notify_geometry_fifo_event();
          }
        }
      }
    }

    for i in 0..2 {
      let dispstat = &mut self.dispstat[i];
      let interrupt_request = &mut interrupt_requests[i];

      if dispstat.flags.contains(DispStatFlags::VCOUNTER_IRQ_ENABLE) && vcount == dispstat.vcount_setting {
        interrupt_request.insert(InterruptRequestRegister::VCOUNTER_MATCH);
      }
    }
    self.thread_data.vcount.store(vcount, Ordering::Release);
  }

  fn start_capture_image(
    dispcapcnt: &DisplayCaptureControlRegister,
    data: &RenderingData,
    frame_buffer: &[Pixel3d; SCREEN_WIDTH as usize * SCREEN_HEIGHT as usize],
    vcount: u16,
    vram: &mut VRam
  ) {
    let width = dispcapcnt.get_capture_width() as usize;
    let start_address = vcount as usize * SCREEN_WIDTH as usize;
    let block = data.dispcnt.vram_block;

    if dispcapcnt.source_b == ScreenSourceB::MainMemoryDisplayFifo {
      todo!("main memory display fifo not implemented");
    }

    fn get_3d_pixel(address: usize, _: &RenderingData, frame_buffer: &[Pixel3d]) -> u16 {
      if let Some(color) = frame_buffer[address].color {
        return (color.r & 0x1f) as u16 | (color.g as u16 & 0x1f) << 5 | (color.b as u16 & 0x1f) << 10
      }
      0
    }

    fn get_pixel(address: usize, data: &RenderingData, _: &[Pixel3d]) -> u16 {
      let r = data.pixels[3 * address] >> 3;
      let g = data.pixels[3 * address + 1] >> 3;
      let b = data.pixels[3 * address + 2] >> 3;

      (r & 0x1f) as u16 | (g as u16 & 0x1f) << 5 | (b as u16 & 0x1f) << 10
    }

    fn get_alpha_3d(address: usize, _: &RenderingData, frame_buffer: &[Pixel3d]) -> u8 {
      if let Some(color) = frame_buffer[address].color {
        if let Some(alpha) = color.alpha {
          if alpha == 0 {
            return 0;
          }
        }
        return 1;
      }
      0
    }

    fn get_alpha(address: usize, data: &RenderingData, _: &[Pixel3d]) -> u8 {
      data.pixel_alphas[address] as u8
    }

    let (source_a, alpha_source):
      (fn(usize, &RenderingData, &[Pixel3d]) -> u16, fn(usize, &RenderingData, &[Pixel3d]) -> u8) =
        if dispcapcnt.source_a == ScreenSourceA::Screen3d || data.dispcnt.display_mode != DisplayMode::Mode0 {
          (get_3d_pixel, get_alpha_3d)
        } else {
          (get_pixel, get_alpha)
        };

    let read_offset = if data.dispcnt.display_mode != DisplayMode::Mode2 {
      2 * start_address + dispcapcnt.vram_read_offset as usize
    } else {
      2 * start_address
    };

    let mut source_b: [u8; 2 * SCREEN_WIDTH as usize] = [0; 2 * SCREEN_WIDTH as usize];

    source_b[..2 * width].copy_from_slice(&vram.banks[block as usize][read_offset..read_offset + 2 * width]);

    let write_offset = 2 * start_address as usize + dispcapcnt.vram_write_offset as usize;
    let write_block = dispcapcnt.vram_write_block as usize;

    fn process_channels(channel_a: u16, channel_b: u16, a_alpha: u16, b_alpha: u16, eva: u16, evb: u16) -> u8 {
      /*
        Dest_Intensity = (  (SrcA_Intensitity * SrcA_Alpha * EVA)
          + (SrcB_Intensitity * SrcB_Alpha * EVB) ) / 16
        */
      ((channel_a * a_alpha * eva + channel_b * b_alpha * evb) / 16) as u8
    }

    // finally transfer the capture image!
    match dispcapcnt.capture_source {
      CaptureSource::SourceA => {
        let mut index = 0;
        for address in start_address..start_address+width {
          let pixel = source_a(address, data, frame_buffer);

          vram.banks[write_block][write_offset + 2 * index] = pixel as u8;
          vram.banks[write_block][write_offset + 2 * index + 1] = (pixel >> 8) as u8;

          index += 1;
        }
      }
      CaptureSource::SourceB => {
        vram.banks[write_block][write_offset..write_offset + 2 * width].copy_from_slice(&source_b[..2 * width]);
      }
      CaptureSource::Blended => {
        let mut index: usize = 0;
        for address_a in start_address..start_address+width {
          let pixel_a = source_a(address_a, data, frame_buffer);

          let alpha_address = if dispcapcnt.source_a == ScreenSourceA::Screen3d || data.dispcnt.display_mode != DisplayMode::Mode0 {
            address_a
          } else {
            index
          };

          let alpha_a = alpha_source(alpha_address, data, frame_buffer);

          let pixel_b = source_b[index] as u16 | (source_b[index] as u16) << 8;

          let alpha_b = (pixel_b >> 15 & 0b1) as u8;

          let r_a = (pixel_a & 0x1f) as u8;
          let g_a = ((pixel_a >> 5) & 0x1f) as u8;
          let b_a = ((pixel_a >> 10) & 0x1f) as u8;

          let r_b = (pixel_b & 0x1f) as u8;
          let g_b = ((pixel_b >> 5) & 0x1f) as u8;
          let b_b = ((pixel_b >> 10) & 0x1f) as u8;


          let new_r = process_channels(
            r_a as u16,
            r_b as u16,
            alpha_a as u16,
            alpha_b as u16,
            dispcapcnt.eva as u16,
            dispcapcnt.evb as u16
          );
          let new_g = process_channels(
            g_a as u16,
            g_b as u16,
            alpha_a as u16,
            alpha_b as u16,
            dispcapcnt.eva as u16,
            dispcapcnt.evb as u16
          );
          let new_b = process_channels(
            b_a as u16,
            b_b as u16,
            alpha_a as u16,
            alpha_b as u16,
            dispcapcnt.eva as u16,
            dispcapcnt.evb as u16
          );
          // Dest_Alpha = (SrcA_Alpha AND (EVA>0)) OR (SrcB_Alpha AND EVB>0))
          let alpha = (alpha_a > 0 && dispcapcnt.eva > 0) || (alpha_b > 0 && dispcapcnt.evb > 0);

          let new_color = (new_r as u16) & 0x1f | ((new_g as u16) & 0x1f) << 5 | ((new_b as u16) & 0x1f) << 10 | (alpha as u16) << 15;

          vram.banks[write_block][write_offset + 2 * index] = new_color as u8;
          vram.banks[write_block][write_offset + 2 * index + 1] = (new_color >> 8) as u8;

          index += 1;
        }
      }
    }
  }

  pub fn write_palette_a<T: Number>(&mut self, address: u32, val: T) {
    self.engine_a.write_palette_ram(address, val);
  }

  pub fn read_palette_a<T: Number>(&self, address: u32) -> T {
    self.engine_a.read_palette_ram(address)
  }

  pub fn read_palette_b<T: Number>(&self, address: u32) -> T {
    self.engine_b.read_palette_ram(address)
  }

  pub fn write_palette_b<T: Number>(&mut self, address: u32, val: T) {
    self.engine_b.write_palette_ram(address, val);
  }

  pub fn write_lcdc<T: Number>(&mut self, address: u32, val: T) {
    match address {
      0x680_0000..=0x681_ffff => self.vram.write_lcdc_bank(Bank::BankA, address, val),
      0x682_0000..=0x683_ffff => self.vram.write_lcdc_bank(Bank::BankB, address, val),
      0x684_0000..=0x685_ffff => self.vram.write_lcdc_bank(Bank::BankC, address, val),
      0x686_0000..=0x687_ffff => self.vram.write_lcdc_bank(Bank::BankD, address, val),
      0x688_0000..=0x688_ffff => self.vram.write_lcdc_bank(Bank::BankE, address, val),
      0x689_0000..=0x689_3fff => self.vram.write_lcdc_bank(Bank::BankF, address, val),
      0x689_4000..=0x689_7fff => self.vram.write_lcdc_bank(Bank::BankG, address, val),
      0x689_8000..=0x689_ffff => self.vram.write_lcdc_bank(Bank::BankH, address, val),
      0x68a_0000..=0x68a_3fff => self.vram.write_lcdc_bank(Bank::BankI, address, val),
      _ => unreachable!("received address: {:X}", address)
    }
  }

  pub fn read_lcdc<T: Number>(&mut self, address: u32) -> T {
    match address {
      0x680_0000..=0x681_ffff => self.vram.read_lcdc_bank(Bank::BankA, address),
      0x682_0000..=0x683_ffff => self.vram.read_lcdc_bank(Bank::BankB, address),
      0x684_0000..=0x685_ffff => self.vram.read_lcdc_bank(Bank::BankC, address),
      0x686_0000..=0x687_ffff => self.vram.read_lcdc_bank(Bank::BankD, address),
      0x688_0000..=0x688_ffff => self.vram.read_lcdc_bank(Bank::BankE, address),
      0x689_0000..=0x689_3fff => self.vram.read_lcdc_bank(Bank::BankF, address),
      0x689_4000..=0x689_7fff => self.vram.read_lcdc_bank(Bank::BankG, address),
      0x689_8000..=0x689_ffff => self.vram.read_lcdc_bank(Bank::BankH, address),
      0x68a_0000..=0x68a_3fff => self.vram.read_lcdc_bank(Bank::BankI, address),
      _ => unreachable!("received address: {:X}", address)
    }
  }

  pub fn read_arm7_wram<T: Number>(&self, address: u32) -> T {
    self.vram.read_arm7_wram(address)
  }

  pub fn write_vramcnt(&mut self, offset: u32, val: u8) {
    if self.vramcnt[offset as usize].vram_enable {
      match offset {
        BANK_A => self.vram.unmap_bank(Bank::BankA, &self.vramcnt[offset as usize]),
        BANK_B => self.vram.unmap_bank(Bank::BankB, &self.vramcnt[offset as usize]),
        BANK_C => self.vram.unmap_bank(Bank::BankC, &self.vramcnt[offset as usize]),
        BANK_D => self.vram.unmap_bank(Bank::BankD, &self.vramcnt[offset as usize]),
        BANK_E => self.vram.unmap_bank(Bank::BankE, &self.vramcnt[offset as usize]),
        BANK_F => self.vram.unmap_bank(Bank::BankF, &self.vramcnt[offset as usize]),
        BANK_G => self.vram.unmap_bank(Bank::BankG, &self.vramcnt[offset as usize]),
        BANK_H => self.vram.unmap_bank(Bank::BankH, &self.vramcnt[offset as usize]),
        BANK_I => self.vram.unmap_bank(Bank::BankI, &self.vramcnt[offset as usize]),
        _ => unreachable!("can't happen")
      }
    }

    self.vramcnt[offset as usize].write(val);

    if self.vramcnt[offset as usize].vram_enable {
      match offset {
        BANK_A => self.vram.map_bank(Bank::BankA, &self.vramcnt[offset as usize]),
        BANK_B => self.vram.map_bank(Bank::BankB, &self.vramcnt[offset as usize]),
        BANK_C => self.vram.map_bank(Bank::BankC, &self.vramcnt[offset as usize]),
        BANK_D => self.vram.map_bank(Bank::BankD, &self.vramcnt[offset as usize]),
        BANK_E => self.vram.map_bank(Bank::BankE, &self.vramcnt[offset as usize]),
        BANK_F => self.vram.map_bank(Bank::BankF, &self.vramcnt[offset as usize]),
        BANK_G => self.vram.map_bank(Bank::BankG, &self.vramcnt[offset as usize]),
        BANK_H => self.vram.map_bank(Bank::BankH, &self.vramcnt[offset as usize]),
        BANK_I => self.vram.map_bank(Bank::BankI, &self.vramcnt[offset as usize]),
        _ => todo!("unimplemented")
      }
    }
  }

  pub fn read_vramcnt(&self, offset: u32) -> u8 {
    self.vramcnt[offset as usize].read()
  }

  pub fn get_arm7_vram_stat(&self) -> u8 {
    ((self.vramcnt[2].vram_enable && self.vramcnt[2].vram_mst == 2) as u8) | ((self.vramcnt[3].vram_enable && self.vramcnt[3].vram_mst == 2) as u8) << 1
  }

  pub fn schedule_hdraw(&mut self, scheduler: &mut Scheduler, cycles_left: usize) {
    scheduler.schedule(EventType::HDraw, HDRAW_CYCLES - cycles_left);
  }
}