use std::cmp::Reverse;

use priority_queue::PriorityQueue;

#[derive(Hash, Eq, PartialEq, Debug)]
pub enum EventType {
  HBLANK,
  NEXT_LINE,
  DMA7(usize),
  DMA9(usize)
}

pub struct Scheduler {
  pub cycles: usize,
  pub queue: PriorityQueue<EventType, Reverse<usize>>
}

impl Scheduler {
  pub fn new() -> Self {
    Self {
      cycles: 0,
      queue: PriorityQueue::new()
    }
  }

  pub fn schedule(&mut self, event_type: EventType, time: usize) {
    self.queue.push(event_type, Reverse(self.cycles + time));
  }

  pub fn update_cycles(&mut self, cycles: usize) {
    self.cycles = cycles;
  }

  pub fn get_next_event(&mut self) -> Option<EventType> {
    let (_, Reverse(cycles)) = self.queue.peek().unwrap();

    if self.cycles >= *cycles {
      let (event_type, _) = self.queue.pop().unwrap();
      return Some(event_type);
    }

    None
  }

  pub fn get_cycles_to_next_event(&mut self) -> usize {
    if let Some((_, Reverse(cycles))) = self.queue.peek() {
      *cycles
    } else {
      0
    }
  }
}

