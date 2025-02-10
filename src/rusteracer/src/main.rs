mod example;
mod judge;
mod opendata;
mod simulation;
mod solve;

use crate::opendata::OpenData;
use crate::simulation::*;

pub fn main() {
    OpenData::new()
        .add_judge(judge::judge)
        // .add_solver("--solve", solve::solve_by_max_reach)
        // .add_solver("--solve-empty", || { println!("0") })
        // .add_solver("--solve-random", solve::solve_random)
        .handle();
}

#[cfg(test)]
mod tests {
    use crate::*;
    use std::fs;
    use std::path::PathBuf;

    fn verify_equal_states(
        simulation: &mut Simulation,
        instructions: &Vec<Instruction>,
        path: PathBuf,
    ) {
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
                    assert_eq!(char, '1', "{}", format!("States differ after line {}", i));
                } else {
                    assert_eq!(char, '0', "{}", format!("States differ after line {}", i));
                }
            }
        }
    }

    fn find_equal_states_cases(dir: &str) -> Vec<(PathBuf, PathBuf, PathBuf)> {
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

    /// Test that the simulation states between implementations are the same.
    #[test]
    fn test_states() {
        for (map, instructions, states) in find_equal_states_cases("../../test/states/") {
            println!("Verifying equal states for '{:?}'", map);

            let mut simulation = Simulation::load(&map);
            let instructions = Instruction::load(&instructions);

            verify_equal_states(&mut simulation, &instructions, states);
        }
    }

    #[test]
    fn test_sample_solutions() {
        // a bit of a misuse since the states path doesn't exist, but works I guess
        for (map, instructions, _) in find_equal_states_cases("../../test/solves/") {
            println!("Verifying solves for '{:?}'", map);

            let mut simulation = Simulation::load(&map);
            let instructions = Instruction::load(&instructions);

            simulation.simulate(&instructions);

            let (vertices, edges, vertex_objects) =
                solve::load_asteroid_graph(&PathBuf::from("../../graphs/sprint.txt"))
                    .ok()
                    .unwrap();

            let (distance, shortest_path) =
                solve::shortest_path(&vertices, &edges, &vertex_objects).unwrap();

            println!("Shortest path: {:?}", shortest_path);

            println!(
                "{:?}",
                solve::closest_distance_to_path(
                    &shortest_path,
                    &vertices,
                    (simulation.racer.x, simulation.racer.y)
                )
            );

            assert!(simulation.finished());
        }
    }

    /// Test that the sample implementation runs. Just shouldn't crash, that's all.
    #[test]
    fn test_example_works() {
        example::main()
    }

    /// Test that we can load the asteroid graphs.
    #[test]
    fn test_loading_asteroid_graph() {
        // total misuse since we're only looking for .txt files
        for (path, _, _) in find_equal_states_cases("../../graphs/") {
            solve::load_asteroid_graph(&path).ok();
        }
    }
}
