
#[derive(Debug)]
pub struct Timer {
  act: f32,
  threshold: f32,
  just_over: bool,
  repeat: bool,
}

impl Timer {
  pub fn new(threshold: f32) -> Self {
    Self { act: 0., threshold, repeat: true, just_over: false }
  }

  // pub fn new_timeout(threshold: f32) -> Self {
  //   Self { act: 0., threshold, repeat: false, just_over: false }
  // }

  // pub fn reset(&mut self) {
  //   self.act = 0.;
  //   self.just_over = false;
  // }

  pub fn is_over(&self) -> bool {
    self.act > self.threshold
  }

  pub fn is_just_over(&self) -> bool {
    self.just_over
  }

  pub fn update(&mut self, dt: f32) {
    if self.is_over() && !self.repeat {
      return;
    }
    let updated_time = self.act + dt;
    let over_threshold = updated_time > self.threshold;

    if self.just_over && !over_threshold {
      self.just_over = false;
    } else if over_threshold && !self.just_over {
      self.just_over = true;
    }
    self.act = if over_threshold && self.repeat { 0. } else { updated_time };
  }
}