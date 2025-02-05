use rand::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

pub mod TickFlag {
    pub const COLLIDED: usize = 1;
    pub const GOAL_REACHED: usize = 2;
}

pub type TickResult = usize;

pub type InstType = i8;
pub type PosType = i64;
pub type SpeedType = i64;
pub type SizeType = i64;

pub static MAX_ACCELERATION: InstType = 127;

pub static DRAG_FRACTION: (SpeedType, SpeedType) = (9, 10);
pub static COLLISION_FRACTION: (SpeedType, SpeedType) = (1, 2);
pub static MAX_COLLISION_RESOLUTIONS: usize = 5;

pub static CELL_SIZE: PosType = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Racer {
    pub x: PosType,
    pub y: PosType,
    pub vx: SpeedType,
    pub vy: SpeedType,
    pub radius: SizeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Asteroid {
    pub x: PosType,
    pub y: PosType,
    pub radius: SizeType,
}

pub type Goal = Asteroid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Instruction {
    pub vx: InstType,
    pub vy: InstType,
}

impl Instruction {
    fn valid(vx: PosType, vy: PosType) -> bool {
        distance_squared(vx, vy, 0, 0) <= (MAX_ACCELERATION as PosType).pow(2)
    }

    pub fn new<T>(vx: T, vy: T) -> Self
    where
        T: Copy + Into<PosType>,
    {
        let vx: i64 = vx.into();
        let vy: i64 = vy.into();

        if !Self::valid(vx, vy) {
            // use float to properly normalize here
            let float_distance = ((vx as f64).powf(2.) + (vy as f64).powf(2.)).powf(1. / 2.);

            let mut vx = ((vx as f64 / float_distance) * MAX_ACCELERATION as f64) as PosType;
            let mut vy = ((vy as f64 / float_distance) * MAX_ACCELERATION as f64) as PosType;

            // if we're still over, decrement both values
            if !Self::valid(vx, vy) {
                vx -= vx.signum();
                vy -= vy.signum();
            }

            return Self { vx: vx as InstType, vy: vy as InstType };
        }

        assert!(Self::valid(vx, vy));

        Self {
            vx: vx as InstType,
            vy: vy as InstType,
        }
    }

    pub fn random() -> Self {
        let mut rng = rand::rng();

        Self {
            vx: rng.random::<InstType>(),
            vy: rng.random::<InstType>(),
        }
    }

    pub fn load(path: &PathBuf) -> Vec<Instruction> {
        let contents = fs::read_to_string(path).expect("Failed reading a file!");
        let mut lines = contents.lines();

        let instruction_count = lines.next().unwrap().parse::<usize>().unwrap();

        let mut instructions = vec![];

        for _ in 0..instruction_count {
            let parts = lines
                .next()
                .expect("No more lines!")
                .split_whitespace()
                .collect::<Vec<&str>>();

            instructions.push(Instruction {
                vx: parts[0].parse::<InstType>().unwrap(),
                vy: parts[1].parse::<InstType>().unwrap(),
            })
        }

        instructions
    }

    pub fn save(path: &PathBuf, instructions: &Vec<Instruction>) {
        let mut file = File::create(path).expect("Failed creating a file!");

        for instruction in instructions {
            file.write_all(format!("{} {}\n", instruction.vx, instruction.vy).as_bytes())
                .expect("Failed writing to file!");
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BoundingBox {
    pub min_x: SizeType,
    pub min_y: SizeType,
    pub max_x: SizeType,
    pub max_y: SizeType,
}

impl BoundingBox {
    pub fn width(&self) -> SizeType {
        self.max_x - self.min_x
    }

    pub fn height(&self) -> SizeType {
        self.max_y - self.min_y
    }
}

/// Squared Euclidean distance; useful for distance checks.
fn distance_squared(x1: PosType, y1: PosType, x2: PosType, y2: PosType) -> PosType {
    (x1 - x2).pow(2) + (y1 - y2).pow(2)
}

/// Plain-old integer Euclidean distance.
///
/// Note: this implementation might break for larger position values, but since
/// the maps are never going to be this large, I'm not fixing it now.
pub fn euclidean_distance(x1: PosType, y1: PosType, x2: PosType, y2: PosType) -> PosType {
    (distance_squared(x1, y1, x2, y2) as f64).sqrt() as PosType
}

#[derive(Debug, Clone)]
pub struct Simulation {
    pub initial_racer: Racer,
    pub racer: Racer,

    pub asteroids: Vec<Asteroid>,
    pub goals: Vec<Goal>,
    pub bbox: BoundingBox,

    pub reached_goals: Vec<bool>,

    _grid: HashMap<(PosType, PosType), Vec<Asteroid>>,
    _cell_size: PosType,
}

///
/// # Examples
/// ```
/// let map_path = PathBuf::from("../../maps/test.txt");
///
/// let mut simulation = Simulation::load(&map_path);
///
/// let mut tick_result: TickResult = 0;
///
/// println!("Running simulation until collision...");
///
/// while tick_result & TickFlag::COLLIDED == 0 {
///     tick_result = simulation.tick(Instruction::new(0, MAX_ACCELERATION));
///
///     println!("{:?}", simulation.racer);
/// }
///
/// println!("Bam!");
/// ```
///
impl Simulation {
    pub fn new(racer: Racer, asteroids: Vec<Asteroid>, goals: Vec<Goal>, bbox: BoundingBox) -> Self {
        let reached_goals = vec![false; goals.len()];

        let mut simulation = Self {
            initial_racer: racer,
            racer,
            asteroids,
            goals,
            bbox,
            reached_goals,
            _grid: HashMap::new(),
            _cell_size: CELL_SIZE,
        };

        for &asteroid in &simulation.asteroids {
            let (min_x, min_y) = simulation.coordinate_to_grid(
                asteroid.x - asteroid.radius - racer.radius,
                asteroid.y - asteroid.radius - racer.radius,
            );

            let (max_x, max_y) = simulation.coordinate_to_grid(
                asteroid.x + asteroid.radius + racer.radius,
                asteroid.y + asteroid.radius + racer.radius,
            );

            for grid_x in min_x..=max_x {
                for grid_y in min_y..=max_y {
                    simulation
                        ._grid
                        .entry((grid_x, grid_y))
                        .or_insert(vec![])
                        .push(asteroid);
                }
            }
        }

        simulation
    }

    fn coordinate_to_grid(&self, x: PosType, y: PosType) -> (PosType, PosType) {
        (x / self._cell_size, y / self._cell_size)
    }

    fn move_racer(&mut self, instruction: Instruction) {
        self.racer.vx = (self.racer.vx * DRAG_FRACTION.0) / DRAG_FRACTION.1;
        self.racer.vy = (self.racer.vy * DRAG_FRACTION.0) / DRAG_FRACTION.1;

        self.racer.vx += instruction.vx as SpeedType;
        self.racer.vy += instruction.vy as SpeedType;

        self.racer.x += self.racer.vx as PosType;
        self.racer.y += self.racer.vy as PosType;
    }

    fn push_from_asteroids(&mut self) -> bool {
        let grid_coordinate = self.coordinate_to_grid(self.racer.x, self.racer.y);

        match self._grid.get(&grid_coordinate) {
            None => false,
            Some(asteroids) => {
                for asteroid in asteroids {
                    // not colliding, nothing to be done
                    if euclidean_distance(self.racer.x, self.racer.y, asteroid.x, asteroid.y)
                        > self.racer.radius + asteroid.radius
                    {
                        continue;
                    }

                    // the vector to push the racer out by
                    let nx = self.racer.x - asteroid.x;
                    let ny = self.racer.y - asteroid.y;

                    // how much to push by
                    let distance =
                        euclidean_distance(self.racer.x, self.racer.y, asteroid.x, asteroid.y);
                    let push_by = distance - (self.racer.radius + asteroid.radius);

                    // the actual push
                    self.racer.x -= (nx * push_by) / distance;
                    self.racer.y -= (ny * push_by) / distance;

                    return true;
                }

                false
            }
        }
    }

    fn push_from_bounding_box(&mut self) -> bool {
        // not pretty but easy to read :)
        let mut collided = false;

        if self.racer.x - self.racer.radius < self.bbox.min_x {
            self.racer.x = self.bbox.min_x + self.racer.radius;
            collided = true;
        }
        if self.racer.x + self.racer.radius > self.bbox.max_x {
            self.racer.x = self.bbox.max_x - self.racer.radius;
            collided = true;
        }
        if self.racer.y - self.racer.radius < self.bbox.min_y {
            self.racer.y = self.bbox.min_y + self.racer.radius;
            collided = true;
        }
        if self.racer.y + self.racer.radius > self.bbox.max_y {
            self.racer.y = self.bbox.max_y - self.racer.radius;
            collided = true;
        }

        collided
    }

    fn check_goal(&mut self) -> bool {
        let mut new_goal_reached = false;

        for (i, goal) in self.goals.iter().enumerate() {
            if euclidean_distance(self.racer.x, self.racer.y, goal.x, goal.y)
                <= (self.racer.radius + goal.radius)
            {
                if !&self.reached_goals[i] {
                    new_goal_reached = true;
                }

                self.reached_goals[i] = true;
            }
        }

        new_goal_reached
    }

    fn resolve_collisions(&mut self) -> bool {
        let mut collided = false;

        for _ in 0..MAX_COLLISION_RESOLUTIONS {
            let mut collided_this_iteration = false;

            if self.push_from_asteroids() {
                collided_this_iteration = true;
                collided = true;
            }

            if self.push_from_bounding_box() {
                collided_this_iteration = true;
                collided = true;
            }

            if !collided_this_iteration {
                break;
            }
        }

        if collided {
            self.racer.vx = (self.racer.vx * COLLISION_FRACTION.0) / COLLISION_FRACTION.1;
            self.racer.vy = (self.racer.vy * COLLISION_FRACTION.0) / COLLISION_FRACTION.1;
        }

        collided
    }

    pub fn finished(&self) -> bool {
        self.reached_goals.iter().all(|v| *v)
    }

    pub fn restart(&mut self) {
        self.racer.x = self.initial_racer.x;
        self.racer.y = self.initial_racer.y;
        self.racer.vx = 0;
        self.racer.vy = 0;

        self.reached_goals.fill(false);
    }

    pub fn tick(&mut self, instruction: Instruction) -> TickResult {
        self.move_racer(instruction);
        let collided = self.resolve_collisions();
        let goal = self.check_goal();

        let mut result: TickResult = 0;

        if collided {
            result |= TickFlag::COLLIDED;
        }

        if goal {
            result |= TickFlag::GOAL_REACHED;
        }

        result
    }

    pub fn simulate(&mut self, instructions: &Vec<Instruction>) -> Vec<TickResult> {
        self.restart();

        let mut results = vec![];

        for instruction in instructions {
            results.push(self.tick(*instruction));
        }

        results
    }

    pub fn load(path: &PathBuf) -> Self {
        let binding = fs::read_to_string(path).unwrap();
        let mut lines = binding.lines();

        let mut parts_fn = || {
            lines
                .next()
                .unwrap()
                .split_whitespace()
                .collect::<Vec<&str>>()
        };

        let racer_parts = parts_fn();

        let racer = Racer {
            x: racer_parts[0].parse::<PosType>().unwrap(),
            y: racer_parts[1].parse::<PosType>().unwrap(),
            radius: racer_parts[2].parse::<SizeType>().unwrap(),
            vx: 0,
            vy: 0,
        };

        let bb_parts = parts_fn();

        let bbox = BoundingBox {
            min_x: bb_parts[0].parse::<SizeType>().unwrap(),
            min_y: bb_parts[1].parse::<SizeType>().unwrap(),
            max_x: bb_parts[2].parse::<SizeType>().unwrap(),
            max_y: bb_parts[3].parse::<SizeType>().unwrap(),
        };

        let asteroid_count = parts_fn()[0].parse::<usize>().unwrap();

        let mut asteroids = vec![];
        for _ in 0..asteroid_count {
            let asteroid_parts = parts_fn();

            asteroids.push(Asteroid {
                x: asteroid_parts[0].parse::<PosType>().unwrap(),
                y: asteroid_parts[1].parse::<PosType>().unwrap(),
                radius: asteroid_parts[2].parse::<PosType>().unwrap(),
            });
        }

        let goal_count = parts_fn()[0].parse::<usize>().unwrap();

        let mut goals = vec![];
        for _ in 0..goal_count {
            let goal_parts = parts_fn();

            goals.push(Asteroid {
                x: goal_parts[0].parse::<PosType>().unwrap(),
                y: goal_parts[1].parse::<PosType>().unwrap(),
                radius: goal_parts[2].parse::<PosType>().unwrap(),
            });
        }

        Self::new(racer, asteroids, goals, bbox)
    }
}
