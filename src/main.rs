use macroquad::prelude::*;
use macroquad::rand::{srand, ChooseRandom};
use macroquad::telemetry::ZoneGuard;
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

const G: f32 = 50.;
const AU: f32 = 150000.;
const SHIP_SIZE: f32 = 10.;
const SHIP_ACCELERATION: f32 = 10.;
const SHIP_ROT_SPEED: f32 = 90.;
const INFO_FONT_SIZE: f32 = 18.;
const TRAIL_CLEANUP_IIME: f32 = 300.;
const PHYSICS_STEP: f32 = 0.02;
const SIMULATION_STEP: f32 = 0.5;
const MAJOR_CB_HILL_RADIUS_COEFICIENT: f32 = 3.;
const DAY_TIME: f32 = 24.;
const TERMINAL_VELOCITY: f32 = 30.;


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

fn apply_gravity_asteroids(asteroids: &[CelestialBodyReference], parent: &CelestialBodyReference, dt: f32) {
  for a in asteroids {
    let mut go_a = a.borrow_mut();
    let go_b = parent.borrow();
    let (vela, _) = gravity_vel(go_a.mov.pos, go_a.mov.mass, go_b.mov.pos, go_b.mov.mass, dt);
    go_a.mov.vel += vela;
  }
}

fn apply_gravity_to_celestial_bodies(celestial_bodies: &[CelestialBodyReference], dt: f32) {
  for i in 0..celestial_bodies.len() {
    let mut go_a = celestial_bodies[i].borrow_mut();
    for j in (i+1)..celestial_bodies.len() {
      let mut go_b = celestial_bodies[j].borrow_mut();
      let (vela, velb) = gravity_vel(go_a.mov.pos, go_a.mov.mass, go_b.mov.pos, go_b.mov.mass, dt);
      go_a.mov.vel += vela;
      go_b.mov.vel += velb;
    }
  }
}

fn apply_gravity_to_ships(ships: &[ShipReference], celestial_bodies: &[CelestialBodyReference], dt: f32) {
  for s in ships {
    s.borrow_mut().apply_gravity(celestial_bodies, dt);
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

fn calculate_hill_radius(parent_pos: Vec2, parent_mass: f32, child_pos: Vec2, child_mass: f32) -> f32 {
  let a = (child_pos - parent_pos).length();
  a * (child_mass / (3. * parent_mass)).cbrt()
}

trait GameObject {
  fn update(&mut self, dt: f32);
  fn draw(&self, focus: Vec2, scale: f32);
}

#[derive(Clone)]
enum CelestialBodyType {
  Star,
  Planet,
  Moon,
  Asteroid,
}

impl CelestialBodyType {
  pub fn min_display_radius(&self) -> f32 {
    match self {
      Self::Star => 15.,
      Self::Planet => 5.,
      Self::Moon => 3.,
      Self::Asteroid => 1.,
    }
  }
}

#[derive(Clone)]
struct CelestialBody {
  mov: Movable,
  radius: f32,
  cb_type: CelestialBodyType,
  hill_radius: f32,
  color: Color,
  name: String,
}

impl CelestialBody {
  pub fn new(pos: Vec2, mass: f32, radius: f32, cb_type: CelestialBodyType, color: Color, name: String) -> Self {
    Self {
      mov: Movable::new(pos, Vec2::ZERO, mass, 0.),
      radius,
      cb_type,
      hill_radius: f32::INFINITY,
      color,
      name,
    }
  }

  pub fn from_parent(parent: &CelestialBody, distance: f32, angle: f32, mass: f32, radius: f32, cb_type: CelestialBodyType, color: Color, name: String) -> Self {
    let (pos, vel) = get_initial_position_and_velocity(parent.mov.mass, distance, angle);
    let mov = Movable::new(parent.mov.pos + pos, parent.mov.vel + vel, mass, 0.);
    let hill_radius = calculate_hill_radius(parent.mov.pos, parent.mov.mass, mov.pos + pos, mov.mass);

    Self {
      mov,
      radius,
      cb_type,
      hill_radius,
      color,
      name,
    }
  }

  pub fn pos_in_hill_radius(&self, pos: &Vec2) -> bool {
    let hr = match self.cb_type {
      CelestialBodyType::Asteroid => self.hill_radius,
      _ => self.hill_radius * MAJOR_CB_HILL_RADIUS_COEFICIENT
    };
    point_in_circle(pos, &self.mov.pos, hr)
  }
}

impl GameObject for CelestialBody {
  fn update(&mut self, dt: f32) {
    self.mov.update(dt);
  }

  fn draw(&self, focus: Vec2, scale: f32) {
    let act_pos = (self.mov.pos - focus) / scale;
    let radius = (self.radius / scale).max(self.cb_type.min_display_radius());
    draw_circle(act_pos.x, act_pos.y, radius, self.color);
    match self.cb_type {
      CelestialBodyType::Asteroid => {},
      _ => {
        // draw_circle_lines(act_pos.x, act_pos.y, self.hill_radius / scale, 1., self.color);
        draw_text(&format!("{}", self.name), act_pos.x - radius / 2., act_pos.y - radius - INFO_FONT_SIZE + 4., INFO_FONT_SIZE, self.color);
      }
    }
  }
}

#[derive(Clone)]
enum ShipState {
  Landed(CelestialBodyReference, Vec2),
  InSpace,
  Destroyed,
}

impl Debug for ShipState {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ShipState::InSpace => {
        write!(f, "In space")
      },
      ShipState::Landed(cb, tv) => {
        write!(f, "Landed on {}, takeoff v: [{:.2}][{:.2}]", cb.borrow().name, tv.x, tv.y)
      },
      ShipState::Destroyed => {
        write!(f, "Destroyed")
      }
    }
  }
}

#[derive(Clone)]
struct Ship {
  mov: Movable,
  state: ShipState,
  store: ShipState,
  fuel: f32,
  max_fuel: f32,
  in_hill_radius_of: Vec<CelestialBodyReference>,
}

impl Ship {
  pub fn new(pos: Vec2, vel: Vec2, fuel: f32) -> Self {
    Self {
      mov: Movable::new(pos, vel, 1., 0.),
      state: ShipState::InSpace,
      store: ShipState::InSpace,
      fuel,
      max_fuel: fuel,
      in_hill_radius_of: vec![]
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
    if self.fuel <= 0. {
      return;
    }

    let vel = rotate_vec2_by_rad(&vec2(1., 0.), self.mov.rot) * SHIP_ACCELERATION * dt / self.mov.mass;
    match self.state {
      ShipState::InSpace => {
        self.mov.vel += vel;
      },
      ShipState::Landed(_, ref mut takeoff_vel) => {
        *takeoff_vel += vel;
      },
      _ => {}
    }
    self.fuel -= (SHIP_ACCELERATION * dt).max(0.);
  }

  pub fn turn_left(&mut self, dt: f32) {
    self.mov.rot -= SHIP_ROT_SPEED.to_radians() * dt;
  }

  pub fn turn_right(&mut self, dt: f32) {
    self.mov.rot += SHIP_ROT_SPEED.to_radians() * dt;
  }

  fn land(&mut self, cb: CelestialBodyReference) {
    let rot = -(self.mov.pos - cb.borrow().mov.pos).angle_between(vec2(1., 0.));
    println!("{} > {}, {}, {}", (self.mov.rot - rot).abs(), 30_f32.to_radians(), (self.mov.vel - cb.borrow().mov.vel).length_squared(), TERMINAL_VELOCITY.powi(2));
    if (self.mov.rot % 360_f32.to_radians() - rot).abs() > 30_f32.to_radians() || (self.mov.vel - cb.borrow().mov.vel).length_squared() > TERMINAL_VELOCITY.powi(2) {
      self.state = ShipState::Destroyed;
      return;
    }

    self.mov.rot = rot;
    self.fuel = self.max_fuel;
    self.state = ShipState::Landed(cb.clone(), Vec2::ZERO);
  }

  fn takeoff(&mut self, takeoff_vel: Vec2) {
    self.state = ShipState::InSpace;
    self.mov.vel += takeoff_vel;
  }

  pub fn process_collision(&mut self, celestial_bodies: &[CelestialBodyReference], dt: f32) {
    match self.state.clone() {
      ShipState::InSpace => {
        for cb in celestial_bodies {
          if self.check_collision(Vec2::ZERO, &cb.borrow(), dt) {
            self.land(cb.clone());
          }
        }
      },
      ShipState::Landed(cb, takeoff_vel) => {
        if !self.check_collision(takeoff_vel, &cb.borrow(), dt) {
          self.takeoff(takeoff_vel);
        }
      },
      _ => {}
    }
  }

  pub fn check_collision(&self, vel: Vec2, cb: &CelestialBody, dt: f32) -> bool {
    let mut m = self.mov.clone();
    m.vel += vel;
    m.update(dt);

    point_in_circle(&m.pos, &cb.mov.pos, cb.radius + SHIP_SIZE / 2.)
  }

  pub fn apply_gravity(&mut self, celestial_bodies: &[CelestialBodyReference], dt: f32) {
    match &self.state {
      ShipState::InSpace | ShipState::Destroyed => {
        self.in_hill_radius_of.clear();
        for cb in celestial_bodies {
          if cb.borrow().pos_in_hill_radius(&self.mov.pos) {
            self.in_hill_radius_of.push(cb.clone());
            let mut cb = cb.borrow_mut();
            let (vela, velb) = gravity_vel(self.mov.pos, self.mov.mass, cb.mov.pos, cb.mov.mass, dt);
            self.mov.vel += vela;
            cb.mov.vel += velb;
          }
        }
      },
      ShipState::Landed(cb, _) => {
        self.mov.vel = cb.borrow().mov.vel;
      }
    }
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
      &format!("{:?}, fuel: {:.2}", self.state, self.fuel),
      act_pos.x,
      act_pos.y - SHIP_SIZE - 2. * INFO_FONT_SIZE + 4.,
      INFO_FONT_SIZE, WHITE
    );
  }
}

fn simulate_hill_radius(ships: &[ShipReference], iterations: usize, dt: f32) -> Vec<TrialElement> {
  let _z = ZoneGuard::new("simulate_hill");
  let mut simulated_trail = vec![];
  'ships: for s in ships {
    let celestial_bodies: Vec<CelestialBodyReference> = s.borrow().in_hill_radius_of.clone();
    let mut s = s.borrow_mut();

    s.save();
    for cb in &celestial_bodies {
      cb.borrow_mut().mov.save();
    }

    for i in 0..iterations {
      apply_gravity_to_celestial_bodies(&celestial_bodies, dt);
      s.apply_gravity(&celestial_bodies, dt);

      for cb in &celestial_bodies {
        cb.borrow_mut().update(dt);
        if i % 5 == 0 || i == iterations - 1 {
          simulated_trail.push(((cb.borrow().mov.pos), cb.borrow().color, Timer::new(10.)));
        }
      }
      s.update(dt);

      let state = s.state.clone();
      if let ShipState::InSpace = state {
        for cb in &celestial_bodies {
          if s.check_collision(Vec2::ZERO, &cb.borrow(), PHYSICS_STEP) {
            simulated_trail.push(((s.mov.pos), ORANGE, Timer::new(10.)));
            s.state = ShipState::Landed(cb.clone(), Vec2::ZERO);
            for cb in &celestial_bodies {
              cb.borrow_mut().mov.load();
            }
            s.load();
            continue 'ships;
          }
        }
        if i % 5 == 0 || i == iterations - 1 {
          simulated_trail.push(((s.mov.pos), YELLOW, Timer::new(10.)));
        }
      }
    }

    for cb in &celestial_bodies {
      cb.borrow_mut().mov.load();
    }
    s.load();

  }

  simulated_trail
}

// fn simulate(ships: &[ShipReference], celestial_bodies: &[CelestialBodyReference], iterations: usize, dt: f32) -> Vec<TrialElement> {
//   let _z = ZoneGuard::new("simulate");
//   for cb in celestial_bodies {
//     cb.borrow_mut().mov.save();
//   }
//   for s in ships {
//     s.borrow_mut().save();
//   }
//   let mut simulated_trail = vec![];

//   for i in 0..iterations {
//     apply_gravity_to_celestial_bodies(celestial_bodies, dt);
//     for s in ships {
//       s.borrow_mut().apply_gravity(&celestial_bodies, dt);
//     }

//     for cb in celestial_bodies {
//       cb.borrow_mut().update(dt);
//       match cb.borrow().cb_type {
//         CelestialBodyType::Asteroid(_) => {},
//         _ => {
//           if i % 5 == 0 || i == iterations - 1 {
//             simulated_trail.push(((cb.borrow().mov.pos), cb.borrow().color, Timer::new(10.)));
//           }
//         }
//       }
//     }
//     'ships: for s in ships {
//       s.borrow_mut().update(dt);
//       let state = s.borrow().state.clone();
//       if let ShipState::InSpace = state {
//         for cb in celestial_bodies {
//           if s.borrow().check_collision(Vec2::ZERO, &cb.borrow(), PHYSICS_STEP) {
//             simulated_trail.push(((s.borrow().mov.pos), ORANGE, Timer::new(10.)));
//             s.borrow_mut().state = ShipState::Landed(cb.clone(), Vec2::ZERO);
//             continue 'ships;
//           }
//         }
//         if i % 5 == 0 || i == iterations - 1 {
//           simulated_trail.push(((s.borrow().mov.pos), YELLOW, Timer::new(10.)));
//         }
//       }
//     }
//   }

//   for cb in celestial_bodies {
//     cb.borrow_mut().mov.load();
//   }
//   for s in ships {
//     s.borrow_mut().load();
//   }

//   simulated_trail
// }

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

fn initialize(seed: u64) -> (CelestialBodyReference, Vec<CelestialBodyReference>, Vec<CelestialBodyReference>, Vec<CelestialBodyReference>, Vec<ShipReference>, ShipReference, Vec<GameObjectReference>) {
  srand(seed);

  let sol_mass = 30000000.;
  let sol_mass_ratio = 2000.;

  let sol = wrap_object(
    CelestialBody::new(
      vec2(screen_width() / 2., screen_height() / 2.),
      sol_mass,
      7000.,
      CelestialBodyType::Star,
      ORANGE,
      "Praxidike".to_owned()
    )
  );
  let planet0 = wrap_object(
    CelestialBody::from_parent(
      &sol.borrow(),
      AU * 0.4,
      get_random_angle(),
      sol_mass / (sol_mass_ratio / 0.05),
      100.,
      CelestialBodyType::Planet,
      BROWN,
      "Ananke".to_owned(),
    )
  );
  let planet1 = wrap_object(
    CelestialBody::from_parent(
      &sol.borrow(),
      AU * 0.7,
      get_random_angle(),
      sol_mass / (sol_mass_ratio / 0.8),
      210.,
      CelestialBodyType::Planet,
      BEIGE,
      "Iocaste".to_owned(),
    )
  );
  let planet2 = wrap_object(
    CelestialBody::from_parent(
      &sol.borrow(),
      AU,
      get_random_angle(),
      sol_mass / sol_mass_ratio,
      300.,
      CelestialBodyType::Planet,
      BLUE,
      "Ganymede".to_owned(),
    )
  );

  let planet2_0 = wrap_object(
    CelestialBody::from_parent(
      &planet2.borrow(),
      planet2.borrow().hill_radius * 0.14,
      get_random_angle(),
      900.,
      80.,
      CelestialBodyType::Moon,
      GRAY,
      "Thebe".to_owned(),
    )
  );
  let planet3 = wrap_object(
    CelestialBody::from_parent(
      &sol.borrow(),
      AU * 1.5,
      get_random_angle(),
      sol_mass / (1000. / 0.8),
      200.,
      CelestialBodyType::Planet,
      RED,
      "Themisto".to_owned(),
    )
  );
  let planet3_0 = wrap_object(
    CelestialBody::from_parent(
      &planet3.borrow(),
      planet3.borrow().hill_radius * 0.09,
      get_random_angle(),
      100.,
      60.,
      CelestialBodyType::Moon,
      GRAY,
      "Kalyke".to_owned(),
    )
  );
  let planet3_1 = wrap_object(
    CelestialBody::from_parent(
      &planet3.borrow(),
      planet3.borrow().hill_radius * 0.15,
      get_random_angle(),
      90.,
      50.,
      CelestialBodyType::Moon,
      GRAY,
      "Mneme".to_owned(),
    )
  );
  let planet4 = wrap_object(
    CelestialBody::from_parent(
      &sol.borrow(),
      AU * 5.3,
      get_random_angle(),
      sol_mass / (sol_mass_ratio / 10.),
      3100.,
      CelestialBodyType::Planet,
      BEIGE,
      "Euanthe".to_owned(),
    )
  );
  let planet4_0 = wrap_object(
    CelestialBody::from_parent(
      &planet4.borrow(),
      planet4.borrow().hill_radius * 0.14,
      get_random_angle(),
      90.,
      75.,
      CelestialBodyType::Moon,
      GRAY,
      "Kale".to_owned(),
    )
  );
  let planet4_1 = wrap_object(
    CelestialBody::from_parent(
      &planet4.borrow(),
      planet4.borrow().hill_radius * 0.23,
      get_random_angle(),
      130.,
      90.,
      CelestialBodyType::Moon,
      GRAY,
      "Eurydome".to_owned(),
    )
  );
  let planet4_2 = wrap_object(
    CelestialBody::from_parent(
      &planet4.borrow(),
      planet4.borrow().hill_radius * 0.31,
      get_random_angle(),
      95.,
      75.,
      CelestialBodyType::Moon,
      GRAY,
      "Sponde".to_owned(),
    )
  );
  let mut all_celestial_bodies: Vec<CelestialBodyReference> = vec![
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
  ];
  let major_celestial_bodies: Vec<CelestialBodyReference> = vec![
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
  ];
  let mut minor_celestial_bodies: Vec<CelestialBodyReference> = vec![];

  let mut game_objects: Vec<GameObjectReference> = vec![
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
  ];

  let cb = major_celestial_bodies.choose().unwrap().clone();
  let (p, v) = get_initial_position_and_velocity(cb.borrow().mov.mass, cb.borrow().radius * 1.5, get_random_angle());
  let ship = wrap_object(
    Ship::new(cb.borrow().mov.pos + p, cb.borrow().mov.vel + v, 1000.)
  );
  game_objects.push(ship.clone());

  let asteroid_belt_distance = AU * 2.7;
  for angle in 0..360 {
    let mut last_distance = 0.;
    let mut last_radius = 0.;
    let asteroid_cnt = rand::gen_range(1, 5);
    for i in 0..asteroid_cnt {
      let angle_increment = rand::gen_range(0., 1.);
      let distance = asteroid_belt_distance + last_distance + last_radius + rand::gen_range(500., 1000.);
      let radius = 10. + rand::gen_range(10., 40.);
      let mass = rand::gen_range(50., 100.);

      let asteroid = wrap_object(
        CelestialBody::from_parent(
          &sol.borrow(),
          distance,
          angle as f32 + angle_increment,
          mass,
          radius,
          CelestialBodyType::Asteroid,
          GRAY,
          format!("Ast {:.1}/{}", angle, i),
        )
      );

      minor_celestial_bodies.push(asteroid.clone());
      all_celestial_bodies.push(asteroid.clone());
      game_objects.push(asteroid.clone());

      last_distance = distance - asteroid_belt_distance;
      last_radius = radius;
    }
  }

  let ships: Vec<ShipReference> = vec![ship.clone()];

  (sol, all_celestial_bodies, major_celestial_bodies, minor_celestial_bodies, ships, ship, game_objects)
}

#[macroquad::main(window_conf)]
async fn main() {
  set_pc_assets_folder("assets");
  let mut seed = 3;
  let mut show_trails = false;

  let (
    mut cb_parent,
    mut all_celestial_bodies,
    mut major_celestial_bodies,
    mut minor_celestial_bodies,
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
  let mut day_count: u32 = 1;
  let mut day_timer = Timer::new(DAY_TIME);

  let mut tick = 1;

  set_camera(&Camera2D::from_display_rect(Rect::new(-screen_width() / 2., -screen_height() / 2., screen_width(), screen_height())));


  loop {
    // let dt = get_frame_time();
    let dt = PHYSICS_STEP;

    trail_elements.retain_mut(|(_p, _c, t)| {
      t.update(dt);
      !t.is_just_over()
    });

    if is_key_released(KeyCode::B) {
      seed = seed + 1;
      (
        cb_parent,
        all_celestial_bodies,
        major_celestial_bodies,
        minor_celestial_bodies,
        ships,
        ship,
        game_objects
      ) = initialize(seed);
      simulated_trail = vec![];
      trail_elements = vec![];
      day_count = 0;
      day_timer = Timer::new(DAY_TIME);
    }
    if is_key_released(KeyCode::Space) {
      show_trails = !show_trails;
    }
    if is_key_released(KeyCode::I) {
      tick = (tick * 2).min(1024);
    }
    if is_key_released(KeyCode::J) {
      tick = (tick / 2).max(1);
    }
    if is_key_released(KeyCode::K) {
      tick = 1;
    }
    {
      let mut ship = ship.borrow_mut();
      if is_key_down(KeyCode::W) {
        ship.throttle_up(dt);
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
    }

    for _ in 0..tick
    {
      apply_gravity_to_celestial_bodies(&major_celestial_bodies, dt);
      apply_gravity_asteroids(&minor_celestial_bodies, &cb_parent, dt);
      apply_gravity_to_ships(&ships, &all_celestial_bodies, dt);

      for go in &game_objects {
        go.borrow_mut().update(dt);
      }
      {
        let _z = ZoneGuard::new("collision");
        for s in &ships {
          s.borrow_mut().process_collision(&all_celestial_bodies, dt);
        }
      }
      day_timer.update(dt);
      if day_timer.is_just_over() {
        day_count += 1;
      }
    }
    focus = ship.borrow().mov.pos;

    trail_emitter_timer.update(dt);
    simulated_trail_timer.update(dt);
    if simulated_trail_timer.is_just_over() {
      // simulated_trail = simulate(&ships, &major_celestial_bodies, 200, SIMULATION_STEP);
      simulated_trail = simulate_hill_radius(&ships, 200, SIMULATION_STEP);
    }
    if trail_emitter_timer.is_just_over() {
      trail_elements.push(((ship.borrow().mov.pos), WHITE, Timer::new(TRAIL_CLEANUP_IIME)));
    }

    {
      let _z = ZoneGuard::new("draw");
      for go in &game_objects {
        go.borrow().draw(focus, scale);
      }
    }

    if show_trails {
      let _z = ZoneGuard::new("show_trails");
      for (te_pos, color, _) in &trail_elements {
        let p = (*te_pos - focus) / scale;
        draw_rectangle(p.x - 2., p.y - 2., 4., 4., *color);
      }
      for (te_pos, color, _) in &simulated_trail {
        let p = (*te_pos - focus) / scale;
        draw_rectangle(p.x - 2., p.y - 2., 4., 4., *color);
      }
    }


    draw_text(&format!("Scale: {}, tick: {}", scale, tick), -screen_width() / 2. + 5., -screen_height() / 2. + 30., 24., WHITE);
    // draw_text(&format!("FPS: {}", get_fps()), -screen_width() / 2. + 5., -screen_height() / 2. + 60., 24., WHITE);
    // draw_text(&format!("Seed: {}", seed), -screen_width() / 2. + 5., -screen_height() / 2. + 90., 24., WHITE);
    draw_text(&format!("Elapsed time: {} days", day_count), screen_width() / 2. - 256., -screen_height() / 2. + 30., 24., WHITE);

    #[cfg(debug_assertions)]
    macroquad_profiler::profiler(Default::default());

    next_frame().await
  }
}

