use macroquad::prelude::*;

#[derive(Clone)]
pub struct Movable {
  pub pos: Vec2,
  pub vel: Vec2,
  pub mass: f32,
  pub rot: f32,
  pub store: (Vec2, Vec2, f32)
}

impl Movable {
  pub fn new(pos: Vec2, vel: Vec2, mass: f32, rot: f32) -> Self {
    Self { pos, vel, mass, rot, store: (pos, vel, rot) }
  }

  pub fn save(&mut self) {
    self.store = (self.pos, self.vel, self.rot);
  }

  pub fn load(&mut self) {
    (self.pos, self.vel, self.rot) = self.store;
  }

  pub fn update(&mut self, dt: f32) {
    self.pos += self.vel * dt;
  }
}