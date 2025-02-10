use crate::simulation::{Instruction, PosType, Simulation};
use rand::{random, Rng, RngCore};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::f64;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

pub fn load_asteroid_graph(
    path: &PathBuf,
) -> io::Result<(
    Vec<(PosType, PosType)>,
    Vec<(usize, usize)>,
    Vec<(char, usize)>,
)> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let contents: Vec<String> = reader
        .lines()
        .filter_map(Result::ok)
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .collect();

    let mut iter = contents.iter();
    let first_line: Vec<usize> = iter
        .next()
        .unwrap()
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();

    let (n_racer, n_asteroid, n_goal, m) =
        (first_line[0], first_line[1], first_line[2], first_line[3]);

    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    let mut vertex_objects = Vec::new();

    // Load vertices
    for i in 0..(n_racer + n_asteroid + n_goal) {
        let line: Vec<i64> = iter
            .next()
            .unwrap()
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        vertices.push((line[0], line[1]));

        if i < n_racer {
            vertex_objects.push(('S', i));
        } else if i < (n_racer + n_asteroid) {
            vertex_objects.push(('A', line[2] as usize));
        } else {
            vertex_objects.push(('G', line[2] as usize));
        }
    }

    // Load edges
    for _ in 0..m {
        let line: Vec<usize> = iter
            .next()
            .unwrap()
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();

        edges.push((line[0], line[1]));
    }

    Ok((vertices, edges, vertex_objects))
}

#[derive(Copy, Clone, PartialEq)]
struct State {
    cost: f64,
    position: usize,
}

impl Eq for State {}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.partial_cmp(&self.cost).unwrap()
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn shortest_path(
    vertices: &Vec<(i64, i64)>,
    edges: &Vec<(usize, usize)>,
    vertex_objects: &Vec<(char, usize)>,
) -> Option<(f64, Vec<usize>)> {
    let start = 0;
    let goals: Vec<usize> = vertex_objects
        .iter()
        .enumerate()
        .filter(|(i, (c, _))| *c == 'G')
        .map(|(i, _)| i)
        .collect();

    let mut dist: Vec<f64> = vec![f64::INFINITY; vertices.len()];
    let mut prev: Vec<Option<usize>> = vec![None; vertices.len()];
    let mut heap = BinaryHeap::new();

    dist[start] = 0.0;
    heap.push(State {
        cost: 0.0,
        position: start,
    });

    while let Some(State { cost, position }) = heap.pop() {
        if goals.contains(&position) {
            let mut path = Vec::new();
            let mut current = Some(position);
            while let Some(pos) = current {
                path.push(pos);
                current = prev[pos];
            }
            path.reverse();
            return Some((cost, path));
        }

        if cost > dist[position] {
            continue;
        }

        for &(u, v) in edges.iter() {
            let neighbor = if u == position {
                v
            } else if v == position {
                u
            } else {
                continue;
            };
            let dx = vertices[position].0 - vertices[neighbor].0;
            let dy = vertices[position].1 - vertices[neighbor].1;
            let next_cost = cost + ((dx * dx + dy * dy) as f64).sqrt();

            if next_cost < dist[neighbor] {
                heap.push(State {
                    cost: next_cost,
                    position: neighbor,
                });
                dist[neighbor] = next_cost;
                prev[neighbor] = Some(position);
            }
        }
    }

    None
}

pub fn closest_distance_to_path(
    path: &Vec<usize>,
    vertices: &Vec<(PosType, PosType)>,
    point: (i64, i64),
) -> f64 {
    let mut min_dist = f64::INFINITY;
    let mut reached_distance = 0.0;
    let mut total_length = 0.0;

    for i in 0..(path.len() - 1) {
        let u = path[i];
        let v = path[i + 1];

        let (x1, y1) = (vertices[u].0 as f64, vertices[u].1 as f64);
        let (x2, y2) = (vertices[v].0 as f64, vertices[v].1 as f64);

        // Compute the segment length
        let segment_length = ((x2 - x1).powf(2.0) + (y2 - y1).powf(2.0)).sqrt();
        total_length += segment_length;

        // Compute the closest distance from point to segment (u, v)
        let px = point.0 as f64;
        let py = point.1 as f64;

        // Vector from u to v
        let dx = x2 - x1;
        let dy = y2 - y1;

        // Vector from u to point
        let dx1 = px - x1;
        let dy1 = py - y1;

        // Dot product to find projection scalar
        let dot = dx * dx1 + dy * dy1;
        let len_sq = dx * dx + dy * dy;

        // Compute the projection of the point onto the line
        let t = if len_sq != 0.0 { dot / len_sq } else { 0.0 };

        let closest_x: f64;
        let closest_y: f64;

        // If the projection falls outside the segment, use the nearest endpoint
        if t < 0.0 {
            closest_x = x1;
            closest_y = y1;
        } else if t > 1.0 {
            closest_x = x2;
            closest_y = y2;
        } else {
            closest_x = x1 + t * dx;
            closest_y = y1 + t * dy;
        }

        // Calculate the distance from the point to the closest point on the segment
        let dist = ((px - closest_x).powf(2.0) + (py - closest_y).powf(2.0)).sqrt();

        // Update the minimum distance
        if min_dist > dist {
            min_dist = dist;

            reached_distance = total_length - segment_length + t.clamp(0.0, 1.0) * segment_length;
        }
    }

    if total_length == 0.0 {
        0.0
    } else {
        reached_distance / total_length
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Individual {
    pub(crate) simulation: Simulation,
    pub(crate) instructions: Vec<Instruction>,
    pub(crate) fitness: f64,
}

impl Individual {
    pub(crate) fn new(simulation: Simulation, instructions: Vec<Instruction>) -> Self {
        Individual {
            simulation,
            instructions,
            fitness: 0.0,
        }
    }

    pub(crate) fn mutate(&mut self) {
        let mut rng = rand::rng();

        let instruction = Instruction::random();

        for _ in 0..(rng.random::<f64>() * 10.0) as usize {
            self.instructions.push(instruction);
            self.simulation.tick(instruction);
        }
    }

    pub(crate) fn evaluate_fitness(
        &mut self,
        path: &Vec<usize>,
        vertices: &Vec<(PosType, PosType)>,
    ) {
        self.fitness =
            closest_distance_to_path(path, vertices, (self.simulation.racer.x, self.simulation.racer.y));
    }
}

pub(crate) fn select_best(population: &mut Vec<Individual>, num_best: usize) {
    population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(Ordering::Equal));
    population.truncate(num_best);
}
