use macroquad::prelude::*;
use macroquad::rand::{srand, ChooseRandom};
use std::cell::RefCell;
use std::fmt::Debug;
use std::mem::{replace};
use std::rc::{Rc};

use movable::Movable;
use timer::Timer;

mod timer;
mod movable;

type GameObjectReference = Rc<RefCell<dyn GameObject>>;
type CelestialBodyReference = Rc<RefCell<CelestialBody>>;
type ShipReference = Rc<RefCell<Ship>>;
type TrialElement = (Vec2, Color, Timer);

const G: f32 = 10.;
const AU: f32 = 149600.;
const SHIP_SIZE: f32 = 10.;
const SHIP_ACCELERATION: f32 = 10.;
const SHIP_ROT_SPEED: f32 = 90.;
const INFO_FONT_SIZE: f32 = 18.;
const TRAIL_CLEANUP_IIME: f32 = 300.;
const PHYSICS_STEP: f32 = 0.02;


fn wrap_object<T>(obj: T) -> Rc<RefCell<T>> {
  Rc::new(RefCell::new(obj))
}

fn rotate_vec2_by_rad(v: &Vec2, rad: f32) -> Vec2 {
  let c = rad.cos();
  let s = rad.sin();
  vec2(c*v.x - s*v.y, s*v.x + c*v.y)
}

fn gravity_vel(a_pos: Vec2, a_mass: f32, b_pos: Vec2, b_mass: f32, dt: f32) -> (Vec2, Vec2) {
  let distance_vector = a_pos - b_pos;
  let force_vec = distance_vector.normalize();
  let distance_length = distance_vector.length_squared();

  (
    -force_vec * b_mass * G / distance_length * dt,
    force_vec * a_mass * G / distance_length * dt,
  )
}

fn apply_gravity(ships: &[ShipReference], celestial_bodies: &[CelestialBodyReference], dt: f32) {
  for i in 0..celestial_bodies.len() {
    for j in (i+1)..celestial_bodies.len() {
      let mut go_a = celestial_bodies[i].borrow_mut();
      let mut go_b = celestial_bodies[j].borrow_mut();
      let (vela, velb) = gravity_vel(go_a.mov.pos, go_a.mov.mass, go_b.mov.pos, go_b.mov.mass, dt);
      go_a.mov.vel += vela;
      go_b.mov.vel += velb;
    }
  }

  for s in ships {
    let ship_state = s.borrow().state.clone();
    match ship_state {
      ShipState::InSpace => {
        for cb in celestial_bodies {
          let mut s = s.borrow_mut();
          let mut cb = cb.borrow_mut();
          let (vela, velb) = gravity_vel(s.mov.pos, s.mov.mass, cb.mov.pos, cb.mov.mass, dt);
          s.mov.vel += vela;
          cb.mov.vel += velb;
        }
      },
      ShipState::Landed(cb, _) => {
        let mut s = s.borrow_mut();
        s.mov.vel = cb.borrow().mov.vel;
      }
    }
  }
}

fn get_initial_position_and_velocity(parent_mass: f32, distance: f32, angle: f32) -> (Vec2, Vec2) {
  let delta_vector = rotate_vec2_by_rad(&vec2(distance, 0.), angle.to_radians());
  let speed = (parent_mass / distance * G).sqrt();
  (delta_vector, delta_vector.perp().normalize() * speed)
}

fn point_in_circle(point: &Vec2, circle: &Vec2, radius: f32) -> bool {
  circle.distance_squared(*point) < (radius).powi(2)
}

// fn calculate_hill_radius(parent_pos: Vec2, parent_mass: f32, child_pos: Vec2, child_mass: f32) -> f32 {
//   let a = (child_pos - parent_pos).length();
//   a * (child_mass / (3. * parent_mass)).cbrt()
// }

trait GameObject {
  fn update(&mut self, dt: f32);
  fn draw(&self, focus: Vec2, scale: f32);
}

#[derive(Clone)]
struct CelestialBody {
  mov: Movable,
  radius: f32,
  min_display_radius: f32,
  color: Color,
  name: String,
}

impl CelestialBody {
  pub fn new(pos: Vec2, mass: f32, radius: f32, min_display_radius: f32, color: Color, name: String) -> Self {
    Self {
      mov: Movable::new(pos, Vec2::ZERO, mass, 0.),
      radius,
      min_display_radius,
      color,
      name,
    }
  }

  pub fn from_parent(parent: &CelestialBody, distance: f32, angle: f32, mass: f32, radius: f32, min_display_radius: f32, color: Color, name: String) -> Self {
    let (pos, vel) = get_initial_position_and_velocity(parent.mov.mass, distance, angle);

    Self {
      mov: Movable::new(parent.mov.pos + pos, parent.mov.vel + vel, mass, 0.),
      radius,
      min_display_radius,
      color,
      name,
    }
  }
}

impl GameObject for CelestialBody {
  fn update(&mut self, dt: f32) {
    self.mov.update(dt);
  }

  fn draw(&self, focus: Vec2, scale: f32) {
    let act_pos = (self.mov.pos - focus) / scale;
    let radius = (self.radius / scale).max(self.min_display_radius);
    draw_circle(act_pos.x, act_pos.y, radius, self.color);
    draw_text(&format!("{}", self.name), act_pos.x - radius / 2., act_pos.y - radius - INFO_FONT_SIZE + 4., INFO_FONT_SIZE, self.color);
  }
}

#[derive(Clone)]
enum ShipState {
  Landed(CelestialBodyReference, Vec2),
  InSpace,
}

impl Debug for ShipState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ShipState::InSpace => {
        write!(f, "In space")
      },
      ShipState::Landed(cb, tv) => {
        write!(f, "Landed on {}, takeoff v: [{:.2}][{:.2}]", cb.borrow().name, tv.x, tv.y)
      }
    }
  }
}

#[derive(Clone)]
struct Ship {
  burn: f32,
  mov: Movable,
  state: ShipState,
  store: ShipState,
}

impl Ship {
  pub fn new(pos: Vec2, vel: Vec2) -> Self {
    Self {
      burn: 0.,
      mov: Movable::new(pos, vel, 1., 0.),
      state: ShipState::InSpace,
      store: ShipState::InSpace,
    }
  }

  pub fn save(&mut self) {
    self.mov.save();
    self.store = self.state.clone();
  }

  pub fn load(&mut self) {
    let stored_data = replace(&mut self.store, ShipState::InSpace);
    self.state = stored_data;
    self.mov.load();
  }

  pub fn throttle_up(&mut self, dt: f32) {
    let vel = rotate_vec2_by_rad(&vec2(1., 0.), self.mov.rot) * 10. * dt / self.mov.mass;
    match self.state {
      ShipState::InSpace => {
        self.mov.vel += vel;
      },
      ShipState::Landed(_, ref mut takeoff_vel) => {
        *takeoff_vel += vel;
      }
    }
  }

  pub fn throttle_down(&mut self, dt: f32) {
    self.burn = (self.burn - SHIP_ACCELERATION * dt).max(0.);
  }

  pub fn turn_left(&mut self, dt: f32) {
    self.mov.rot -= SHIP_ROT_SPEED.to_radians() * dt;
  }

  pub fn turn_right(&mut self, dt: f32) {
    self.mov.rot += SHIP_ROT_SPEED.to_radians() * dt;
  }

  pub fn process_collision(&mut self, celestial_bodies: &[CelestialBodyReference], dt: f32) {
    match self.state.clone() {
      ShipState::InSpace => {
        for cb in celestial_bodies {
          if self.check_collision(Vec2::ZERO, &cb.borrow(), dt) {
            self.state = ShipState::Landed(cb.clone(), Vec2::ZERO);
            self.mov.rot = -(self.mov.pos - cb.borrow().mov.pos).angle_between(vec2(1., 0.));
          }
        }
      },
      ShipState::Landed(cb, takeoff_vel) => {
        if !self.check_collision(takeoff_vel, &cb.borrow(), dt) {
          self.state = ShipState::InSpace;
          self.mov.vel += takeoff_vel;
        }
      }
    }
  }

  pub fn check_collision(&self, vel: Vec2, cb: &CelestialBody, dt: f32) -> bool {
    let mut m = self.mov.clone();
    m.vel += vel;
    m.update(dt);

    point_in_circle(&m.pos, &cb.mov.pos, cb.radius + SHIP_SIZE / 2.)
  }
}

impl GameObject for Ship {
  fn update(&mut self, dt: f32) {
    // self.mov.vel += rotate_vec2_by_rad(&vec2(1., 0.), self.mov.rot) * self.burn * dt / self.mov.mass;
    self.mov.update(dt);
  }

  fn draw(&self, focus: Vec2, scale: f32) {
    let v = vec2((SHIP_SIZE / scale).max(3.), 0.);
    let act_pos = (self.mov.pos - focus) / scale;
    let vel = self.mov.vel / scale;
    let (v1, v2, v3) = (
      act_pos + rotate_vec2_by_rad(&v, self.mov.rot),
      act_pos + rotate_vec2_by_rad(&v, self.mov.rot + 135_f32.to_radians()),
      act_pos + rotate_vec2_by_rad(&v, self.mov.rot - 135_f32.to_radians()),
    );
    draw_triangle_lines(v1, v2, v3, 2., WHITE);
    draw_line(
      act_pos.x,
      act_pos.y,
      act_pos.x + vel.x,
      act_pos.y + vel.y,
      2., WHITE
    );
    draw_text(
      &format!("|v|: {:.2}, v: [{:.2}][{:.2}]", self.mov.vel.length(), self.mov.vel.x, self.mov.vel.y),
      act_pos.x,
      act_pos.y - SHIP_SIZE - INFO_FONT_SIZE + 4.,
      INFO_FONT_SIZE, WHITE
    );
    draw_text(
      &format!("state: {:?}", self.state),
      act_pos.x,
      act_pos.y - SHIP_SIZE - 2. * INFO_FONT_SIZE + 4.,
      INFO_FONT_SIZE, WHITE
    );
  }
}

fn simulate(ships: &[ShipReference], celestial_bodies: &[CelestialBodyReference], iterations: usize, dt: f32) -> Vec<TrialElement> {
  for cb in celestial_bodies {
    cb.borrow_mut().mov.save();
  }
  for s in ships {
    s.borrow_mut().save();
  }
  let mut simulated_trail = vec![];

  for i in 0..iterations {
    apply_gravity(ships, celestial_bodies, dt);

    for cb in celestial_bodies {
      cb.borrow_mut().update(dt);
      if i % 5 == 0 || i == iterations - 1 {
        simulated_trail.push(((cb.borrow().mov.pos), cb.borrow().color, Timer::new(10.)));
      }
    }
    'ships: for s in ships {
      s.borrow_mut().update(dt);
      let state = s.borrow().state.clone();
      if let ShipState::InSpace = state {
        for cb in celestial_bodies {
          if s.borrow().check_collision(Vec2::ZERO, &cb.borrow(), PHYSICS_STEP) {
            simulated_trail.push(((s.borrow().mov.pos), ORANGE, Timer::new(10.)));
            s.borrow_mut().state = ShipState::Landed(cb.clone(), Vec2::ZERO);
            continue 'ships;
          }
        }
        if i % 5 == 0 || i == iterations - 1 {
          simulated_trail.push(((s.borrow().mov.pos), YELLOW, Timer::new(10.)));
        }
      }
    }
  }

  for cb in celestial_bodies {
    cb.borrow_mut().mov.load();
  }
  for s in ships {
    s.borrow_mut().load();
  }

  simulated_trail
}

fn window_conf() -> Conf {
  Conf {
    window_title: "solsys".to_owned(),
    window_width: 1320,
    window_height: 760,
    high_dpi: false,
    ..Default::default()
  }
}

fn get_scale_delta(scale: f32) -> f32 {
  if scale >= 200. {
    return 100.;
  }
  if scale >= 100. {
    return 20.;
  }
  if scale >= 50. {
    return 10.;
  }
  if scale >= 10. {
    return 2.;
  }
  return 0.5;
}

fn get_random_angle() -> f32 {
  rand::gen_range(-180., 180.)
}

fn initialize(seed: u64) -> (Vec<CelestialBodyReference>, Vec<ShipReference>, ShipReference, Vec<GameObjectReference>) {
  srand(seed);

  let sol_mass = 30000000.;

  let sol = wrap_object(
    CelestialBody::new(
      vec2(screen_width() / 2., screen_height() / 2.),
      sol_mass,
      7000.,
      15.,
      ORANGE,
      "Praxidike".to_owned()
    )
  );
  let planet0 = wrap_object(
    CelestialBody::from_parent(
      &sol.borrow(),
      AU * 0.4,
      get_random_angle(),
      sol_mass / (3300. / 0.05),
      100.,
      5.,
      BROWN,
      "Ananke".to_owned(),
    )
  );
  let planet1 = wrap_object(
    CelestialBody::from_parent(
      &sol.borrow(),
      AU * 0.7,
      get_random_angle(),
      sol_mass / (3300. / 0.8),
      210.,
      5.,
      BEIGE,
      "Iocaste".to_owned(),
    )
  );
  let planet2 = wrap_object(
    CelestialBody::from_parent(
      &sol.borrow(),
      AU,
      get_random_angle(),
      sol_mass / 3300.,
      300.,
      5.,
      BLUE,
      "Ganymede".to_owned(),
    )
  );
  let planet2_0 = wrap_object(
    CelestialBody::from_parent(
      &planet2.borrow(),
      4000.,
      get_random_angle(),
      900.,
      80.,
      2.,
      GRAY,
      "Thebe".to_owned(),
    )
  );
  let planet3 = wrap_object(
    CelestialBody::from_parent(
      &sol.borrow(),
      AU * 1.5,
      get_random_angle(),
      sol_mass / (3300. / 0.5),
      200.,
      5.,
      RED,
      "Themisto".to_owned(),
    )
  );
  let planet3_0 = wrap_object(
    CelestialBody::from_parent(
      &planet3.borrow(),
      3500.,
      get_random_angle(),
      600.,
      60.,
      2.,
      GRAY,
      "Kalyke".to_owned(),
    )
  );
  let planet3_1 = wrap_object(
    CelestialBody::from_parent(
      &planet3.borrow(),
      6000.,
      get_random_angle(),
      500.,
      50.,
      2.,
      GRAY,
      "Mneme".to_owned(),
    )
  );
  let planet4 = wrap_object(
    CelestialBody::from_parent(
      &sol.borrow(),
      AU * 3.5,
      get_random_angle(),
      sol_mass / 1000.,
      3100.,
      5.,
      BEIGE,
      "Euanthe".to_owned(),
    )
  );
  let planet4_0 = wrap_object(
    CelestialBody::from_parent(
      &planet4.borrow(),
      7000.,
      get_random_angle(),
      900.,
      75.,
      2.,
      GRAY,
      "Kale".to_owned(),
    )
  );
  let planet4_1 = wrap_object(
    CelestialBody::from_parent(
      &planet4.borrow(),
      12900.,
      get_random_angle(),
      1300.,
      120.,
      2.,
      GRAY,
      "Eurydome".to_owned(),
    )
  );
  let planet4_2 = wrap_object(
    CelestialBody::from_parent(
      &planet4.borrow(),
      17700.,
      get_random_angle(),
      950.,
      150.,
      2.,
      GRAY,
      "Sponde".to_owned(),
    )
  );
  let planet4_3 = wrap_object(
    CelestialBody::from_parent(
      &planet4.borrow(),
      29000.,
      get_random_angle(),
      600.,
      50.,
      2.,
      GRAY,
      "S/2003".to_owned(),
    )
  );
  let celestial_bodies: Vec<CelestialBodyReference> = vec![
    sol.clone(),
    planet0.clone(),
    planet1.clone(),
    planet2.clone(),
    planet2_0.clone(),
    planet3.clone(),
    planet3_0.clone(),
    planet3_1.clone(),
    planet4.clone(),
    planet4_0.clone(),
    planet4_1.clone(),
    planet4_2.clone(),
    planet4_3.clone(),
  ];

  let cb = celestial_bodies.choose().unwrap().clone();
  let (p, v) = get_initial_position_and_velocity(cb.borrow().mov.mass, cb.borrow().radius * 1.5, get_random_angle());
  let ship = wrap_object(
    Ship::new(cb.borrow().mov.pos + p, cb.borrow().mov.vel + v)
  );

  let game_objects: Vec<GameObjectReference> = vec![
    sol.clone(),
    planet0.clone(),
    planet1.clone(),
    planet2.clone(),
    planet2_0.clone(),
    planet3.clone(),
    planet3_0.clone(),
    planet3_1.clone(),
    planet4.clone(),
    planet4_0.clone(),
    planet4_1.clone(),
    planet4_2.clone(),
    planet4_3.clone(),
    ship.clone()
  ];

  let ships: Vec<ShipReference> = vec![ship.clone()];

  (celestial_bodies, ships, ship, game_objects)
}

#[macroquad::main(window_conf)]
async fn main() {
  set_pc_assets_folder("assets");
  let mut seed = 3;

  let (
    mut celestial_bodies,
    mut ships,
    mut ship,
    mut game_objects
  ) = initialize(seed);

  let mut focus;
  let mut scale = 1.;
  let mut trail_emitter_timer = Timer::new(2.);
  let mut trail_elements: Vec<TrialElement> = vec![];
  let mut simulated_trail_timer = Timer::new(0.5);
  let mut simulated_trail: Vec<TrialElement> = vec![];

  set_camera(&Camera2D::from_display_rect(Rect::new(-screen_width() / 2., -screen_height() / 2., screen_width(), screen_height())));


  loop {
    // let dt = get_frame_time();
    let dt = PHYSICS_STEP;
    apply_gravity(&ships, &celestial_bodies, dt);
    trail_elements.retain_mut(|(_p, _c, t)| {
      t.update(dt);
      !t.is_just_over()
    });

    if is_key_released(KeyCode::I) {
      seed = seed + 1;
      (
        celestial_bodies,
        ships,
        ship,
        game_objects
      ) = initialize(seed);
      simulated_trail = vec![];
      trail_elements = vec![];
    }
    {
      let mut ship = ship.borrow_mut();
      if is_key_down(KeyCode::W) {
        ship.throttle_up(dt);
      }
      if is_key_down(KeyCode::S) {
        ship.throttle_down(dt);
      }
      if is_key_down(KeyCode::A) {
        ship.turn_left(dt);
      }
      if is_key_down(KeyCode::D) {
        ship.turn_right(dt);
      }
      if is_key_released(KeyCode::X) {
        scale = 1.;
      }
      if mouse_wheel().1 > 0. {
        scale = (scale - get_scale_delta(scale)).max(0.5);
      } else if mouse_wheel().1 < 0. {
        scale = (scale + get_scale_delta(scale)).min(5000.);
      }
      focus = ship.mov.pos;
      // focus = planet2.borrow().mov.pos;
    }

    for go in &game_objects {
      go.borrow_mut().update(dt);
    }
    for s in &ships {
      s.borrow_mut().process_collision(&celestial_bodies, dt);
    }

    trail_emitter_timer.update(dt);
    simulated_trail_timer.update(dt);

    if simulated_trail_timer.is_just_over() {
      simulated_trail = simulate(&ships, &celestial_bodies, 200, 0.5);
    }
    if trail_emitter_timer.is_just_over() {
      trail_elements.push(((ship.borrow().mov.pos), WHITE, Timer::new(TRAIL_CLEANUP_IIME)));
    }

    for go in &game_objects {
      go.borrow().draw(focus, scale);
    }

    for (te_pos, color, _) in &trail_elements {
      let p = (*te_pos - focus) / scale;
      draw_circle(p.x, p.y, 2., *color);
    }
    for (te_pos, color, _) in &simulated_trail {
      let p = (*te_pos - focus) / scale;
      draw_circle(p.x, p.y, 2., *color);
    }

    draw_text(&format!("Scale: {}", scale), -screen_width() / 2. + 5., -screen_height() / 2. + 30., 24., WHITE);
    draw_text(&format!("FPS: {}", get_fps()), -screen_width() / 2. + 5., -screen_height() / 2. + 60., 24., WHITE);
    draw_text(&format!("Seed: {}", seed), -screen_width() / 2. + 5., -screen_height() / 2. + 90., 24., WHITE);
    #[cfg(debug_assertions)]
    macroquad_profiler::profiler(Default::default());

    next_frame().await
  }
}

