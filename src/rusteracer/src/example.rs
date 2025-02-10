use std::cmp::Ordering;
use std::path::PathBuf;

use crate::simulation::*;
use crate::solve;

pub fn main() {
    let mut simulation = Simulation::load(&PathBuf::from("../../maps/sprint.txt"));

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

    let population_size = 10;
    let generations = 1000;
    let mutation_count = 10;

    let mut population: Vec<solve::Individual> = (0..population_size)
        .map(|_| solve::Individual::new(simulation.clone(), vec![]))
        .collect();

    let mut max_fitness: f64 = 0.0;

    for i in 0..generations {
        let mut new_population: Vec<solve::Individual> = Vec::new();

        // For each individual, mutate K times and add to the new population
        for individual in &population {
            for _ in 0..mutation_count {
                let mut mutated_individual = individual.clone();
                mutated_individual.mutate();
                new_population.push(mutated_individual);
            }
        }

        // Evaluate fitness for all individuals in the new population
        for individual in &mut new_population {
            individual.evaluate_fitness(&shortest_path, &vertices);
        }

        // Combine original population with new mutated individuals
        let mut combined_population = population.clone();
        combined_population.append(&mut new_population);

        // Select the best individuals to form the next generation
        solve::select_best(&mut combined_population, population_size);

        // Update population to the best individuals
        population = combined_population;

        // Output the best individual
        if let Some(best) = population
            .iter()
            .max_by(|a, b| a.fitness.partial_cmp(&b.fitness).unwrap_or(Ordering::Equal))
        {
            if best.fitness > max_fitness {
                max_fitness = best.fitness;
                println!("[{}] Better max fitness: {}", i, max_fitness);

                Instruction::save(&PathBuf::from("../../best.txt"), &best.instructions)
            }
        }
    }
}
