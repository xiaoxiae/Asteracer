use rand::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

mod TickFlag {
    pub const Collided: usize = 1;
    pub const GoalReached: usize = 2;
}

type TickResult = usize;

type InstType = i8;
type PosType = i64;
type SpeedType = i64;
type SizeType = i64;

static MAX_ACCELERATION: InstType = 127;

static DRAG_FRACTION: (SpeedType, SpeedType) = (9, 10);
static COLLISION_FRACTION: (SpeedType, SpeedType) = (1, 2);
static MAX_COLLISION_RESOLUTIONS: usize = 5;

static CELL_SIZE: PosType = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Racer {
    x: PosType,
    y: PosType,
    vx: SpeedType,
    vy: SpeedType,
    radius: SizeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Asteroid {
    x: PosType,
    y: PosType,
    radius: SizeType,
}

type Goal = Asteroid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Instruction {
    vx: InstType,
    vy: InstType,
}

impl Instruction {
    fn valid(vx: InstType, vy: InstType) -> bool {
        let vx = vx as PosType;
        let vy = vy as PosType;

        distance_squared(vx, vy, 0, 0) <= (MAX_ACCELERATION as PosType).pow(2)
    }

    fn new(vx: InstType, vy: InstType) -> Self {
        if !Self::valid(vx, vy) {
            // use float to properly normalize here
            let distance = (vx.pow(2) as f64 + vy.pow(2) as f64).powf(1. / 2.);

            let mut vx = ((vx as f64 / distance) * MAX_ACCELERATION as f64) as InstType;
            let mut vy = ((vy as f64 / distance) * MAX_ACCELERATION as f64) as InstType;

            // if we're still over, decrement both values
            if !Self::valid(vx, vy) {
                vx -= vx.signum();
                vy -= vy.signum();
            }

            return Self { vx, vy };
        }

        assert!(Self::valid(vx, vy));

        Self {
            vx: vx as InstType,
            vy: vy as InstType,
        }
    }

    fn random() -> Self {
        let mut rng = rand::rng();

        Self {
            vx: rng.random::<InstType>(),
            vy: rng.random::<InstType>(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BoundingBox {
    min_x: SizeType,
    min_y: SizeType,
    max_x: SizeType,
    max_y: SizeType,
}

impl BoundingBox {
    fn width(&self) -> SizeType {
        self.max_x - self.min_x
    }

    fn height(&self) -> SizeType {
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
fn euclidean_distance(x1: PosType, y1: PosType, x2: PosType, y2: PosType) -> PosType {
    (distance_squared(x1, y1, x2, y2) as f64).sqrt() as PosType
}

#[derive(Debug, Clone)]
struct Simulation {
    initial_racer: Racer,
    racer: Racer,

    asteroids: Vec<Asteroid>,
    goals: Vec<Goal>,
    bbox: BoundingBox,

    reached_goals: Vec<bool>,
    pushed_states: Vec<(Racer, Vec<bool>)>,

    _grid: HashMap<(PosType, PosType), Vec<Asteroid>>,
    _cell_size: PosType,
}

impl Simulation {
    fn new(racer: Racer, asteroids: Vec<Asteroid>, goals: Vec<Goal>, bbox: BoundingBox) -> Self {
        let reached_goals = vec![false; goals.len()];

        let mut simulation = Self {
            initial_racer: racer,
            racer,
            asteroids,
            goals,
            bbox,
            reached_goals,
            pushed_states: Vec::new(),
            _grid: HashMap::new(),
            _cell_size: CELL_SIZE,
        };

        for &asteroid in &simulation.asteroids {
            let (min_x, min_y) = simulation._coordinate_to_grid(
                asteroid.x - asteroid.radius - racer.radius,
                asteroid.y - asteroid.radius - racer.radius,
            );

            let (max_x, max_y) = simulation._coordinate_to_grid(
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

    fn _coordinate_to_grid(&self, x: PosType, y: PosType) -> (PosType, PosType) {
        (x / self._cell_size, y / self._cell_size)
    }

    fn _move_racer(&mut self, instruction: Instruction) {
        self.racer.vx = (self.racer.vx * DRAG_FRACTION.0) / DRAG_FRACTION.1;
        self.racer.vy = (self.racer.vy * DRAG_FRACTION.0) / DRAG_FRACTION.1;

        self.racer.vx += instruction.vx as SpeedType;
        self.racer.vy += instruction.vy as SpeedType;

        self.racer.x += self.racer.vx as PosType;
        self.racer.y += self.racer.vy as PosType;
    }

    fn _push_from_asteroids(&mut self) -> bool {
        let grid_coordinate = self._coordinate_to_grid(self.racer.x, self.racer.y);

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

    fn _push_from_bounding_box(&mut self) -> bool {
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

    fn _check_goal(&mut self) -> bool {
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

    fn _resolve_collisions(&mut self) -> bool {
        let mut collided = false;

        for _ in 0..MAX_COLLISION_RESOLUTIONS {
            let mut collided_this_iteration = false;

            if self._push_from_asteroids() {
                collided_this_iteration = true;
                collided = true;
            }

            if self._push_from_bounding_box() {
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

    fn finished(&self) -> bool {
        self.reached_goals.iter().all(|v| *v)
    }

    fn restart(&mut self) {
        self.racer.x = self.initial_racer.x;
        self.racer.y = self.initial_racer.y;
        self.racer.vx = 0;
        self.racer.vy = 0;

        self.reached_goals.fill(false);
    }

    fn tick(&mut self, instruction: Instruction) -> TickResult {
        self._move_racer(instruction);
        let collided = self._resolve_collisions();
        let goal = self._check_goal();

        let mut result: TickResult = 0;

        if collided {
            result |= TickFlag::Collided;
        }

        if goal {
            result |= TickFlag::GoalReached;
        }

        result
    }

    fn simulate(&mut self, instructions: Vec<Instruction>) -> Vec<TickResult> {
        self.restart();

        let mut results = vec![];

        for instruction in instructions {
            results.push(self.tick(instruction));
        }

        results
    }

    fn load(path: &PathBuf) -> Self {
        // TODO: factor out duplicates

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

fn save_instructions(path: &PathBuf, instructions: &Vec<Instruction>) {
    let mut file = File::create(path).expect("Failed creating a file!");

    for instruction in instructions {
        file.write_all(format!("{} {}\n", instruction.vx, instruction.vy).as_bytes())
            .expect("Failed writing to file!");
    }
}

fn load_instructions(path: &PathBuf) -> Vec<Instruction> {
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

/// Sample usage -- fly into an asteroid.
fn main() {
    let map_path = PathBuf::from("../../maps/test.txt");

    let mut simulation = Simulation::load(&map_path);

    let mut tick_result: TickResult = 0;

    println!("Running simulation until collision...");

    while tick_result & TickFlag::Collided == 0 {
        tick_result = simulation.tick(Instruction::new(0, MAX_ACCELERATION));

        println!("{:?}", simulation.racer);
    }

    println!("Bam!");
}

#[cfg(test)]
mod tests {
    use crate::*;
    use std::fs;
    use std::path::PathBuf;

    fn verify_states(simulation: &mut Simulation, instructions: &Vec<Instruction>, path: PathBuf) {
        let contents = fs::read_to_string(&path);

        for (i, (state, &instruction)) in contents.unwrap().lines().zip(instructions).enumerate() {
            let parts = state.split_whitespace().collect::<Vec<&str>>();

            simulation.tick(instruction);

            assert_eq!(parts[0].parse::<PosType>().unwrap(), simulation.racer.x);
            assert_eq!(parts[1].parse::<PosType>().unwrap(), simulation.racer.y);
            assert_eq!(parts[2].parse::<PosType>().unwrap(), simulation.racer.vx);
            assert_eq!(parts[3].parse::<PosType>().unwrap(), simulation.racer.vy);

            for (char, goal) in parts[4].chars().zip(&simulation.reached_goals) {
                if *goal {
                    assert_eq!(char, '1');
                } else {
                    assert_eq!(char, '0');
                }
            }
        }
    }

    fn find_test_cases(dir: &str) -> Vec<(PathBuf, PathBuf, PathBuf)> {
        let mut samples = vec![];

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();

                    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                        if file_name.ends_with("txt") {
                            let map_file = path.clone();
                            let input_file = path.with_extension("in");
                            let output_file = path.with_extension("out");

                            samples.push((map_file, input_file, output_file))
                        }
                    }
                }
            }
        }

        samples
    }

    #[test]
    fn test() {
        for (map, instructions, states) in find_test_cases("../../test/") {
            println!("Verifying '{:?}'", map);

            let mut simulation = Simulation::load(&map);
            let instructions = load_instructions(&instructions);

            verify_states(&mut simulation, &instructions, states);
        }
    }
}
