use std::sync::Arc;

use crate::gpu::ThreadData;

pub struct Renderer2d {
  pub thread_data: Arc<ThreadData>
}