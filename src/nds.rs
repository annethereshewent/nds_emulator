use std::{cell::RefCell, rc::Rc};

use crate::{cpu::{bus::Bus, CPU}, scheduler::{EventType, Scheduler}};

pub struct Nds {
  pub arm9_cpu: CPU<true>,
  pub arm7_cpu: CPU<false>,
  scheduler: Rc<RefCell<Scheduler>>,
  pub bus: Rc<RefCell<Bus>>
}

impl Nds {
  pub fn new(firmware_bytes: Vec<u8>, bios7_bytes: Vec<u8>, bios9_bytes: Vec<u8>, rom_bytes: Vec<u8>, skip_bios: bool) -> Self {
    let mut scheduler = Rc::new(RefCell::new(Scheduler::new()));
    let bus = Rc::new(
      RefCell::new(
        Bus::new(
          firmware_bytes,
          bios7_bytes,
          bios9_bytes,
          rom_bytes,
          skip_bios,
          scheduler.clone()
        )
      )
    );
    let mut nds = Self {
      arm9_cpu: CPU::new(bus.clone(), skip_bios),
      arm7_cpu: CPU::new(bus.clone(), skip_bios),
      scheduler,
      bus
    };

    nds.arm7_cpu.reload_pipeline32();
    nds.arm9_cpu.reload_pipeline32();

    nds
  }

  pub fn step(&mut self) -> bool {
    let ref mut scheduler = *self.scheduler.borrow_mut();

    let cycles = scheduler.get_cycles_to_next_event();

    let actual_target = std::cmp::min(scheduler.cycles + 30, cycles);

    self.arm9_cpu.step(actual_target * 2);
    self.arm7_cpu.step(actual_target);


    scheduler.update_cycles(actual_target);

    let ref mut bus = *self.bus.borrow_mut();

    // finally check if there are any events to handle.
    if let Some(event_type) = scheduler.get_next_event() {
      let mut interrupt_requests = [&mut bus.arm7.interrupt_request, &mut bus.arm9.interrupt_request];
      let mut dma_channels = [&mut bus.arm7.dma_channels, &mut bus.arm9.dma_channels];

      match event_type {
        EventType::HBLANK => bus.gpu.handle_hblank(scheduler, &mut interrupt_requests, &mut dma_channels),
        EventType::NEXT_LINE => bus.gpu.start_next_line(scheduler, &mut interrupt_requests, &mut dma_channels),
        EventType::DMA7(channel_id) => bus.arm7.dma_channels.channels[channel_id].pending = true,
        EventType::DMA9(channel_id) => bus.arm9.dma_channels.channels[channel_id].pending = true,
      }
    }

    bus.gpu.frame_finished
  }
}