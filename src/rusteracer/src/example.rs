use std::path::PathBuf;

use crate::simulation::*;

pub fn main() {
    let mut simulation = Simulation::load(&PathBuf::from("../../maps/test.txt"));

    println!(
        "Starting racer position: {} {}",
        simulation.racer.x, simulation.racer.y
    );
    println!("Number of asteroids: {}", simulation.asteroids.len());
    println!("Number of goals: {}", simulation.goals.len());
    println!();

    // Fly to the right until we hit the wall
    let mut tick = 0;
    println!("Flying to the right...");
    loop {
        let result = simulation.tick(Instruction::new(InstType::MAX, 0));
        if (result & TickFlag::COLLIDED) != 0 {
            println!("We collided after {} ticks! Ouch...", tick);
            println!(
                "Current racer position: {} {}",
                simulation.racer.x, simulation.racer.y
            );
            println!();
            break;
        }
        tick += 1;
    }

    // Fly down to reach the first checkpoint
    println!("Flying down...");
    loop {
        let result = simulation.tick(Instruction::new(0, InstType::MAX));

        println!("{:?}", Instruction::new(0, InstType::MAX));

        if (result & TickFlag::GOAL_REACHED) != 0 {
            println!("We collected a checkpoint after {} ticks!", tick);
            println!("Checkpoints obtained: {:?}", simulation.reached_goals);
            println!(
                "Current racer position: {} {}",
                simulation.racer.x, simulation.racer.y
            );
            println!();
            break;
        }
        tick += 1;
    }

    // Collect all goals by always flying to the nearest one
    while simulation.reached_goals.iter().any(|&reached| !reached) {
        let mut nearest_goal = None;
        let mut nearest_goal_distance = PosType::MAX;

        for (i, &reached) in simulation.reached_goals.iter().enumerate() {
            if !reached {
                let goal = simulation.goals[i];
                let distance = euclidean_distance(goal.x, goal.y, simulation.racer.x, simulation.racer.y);

                if distance < nearest_goal_distance {
                    nearest_goal_distance = distance;
                    nearest_goal = Some(goal);
                }
            }
        }

        let nearest_goal = nearest_goal.unwrap();
        println!("Flying to the nearest goal in a straight line...");
        let mut collided_count = 0;

        loop {
            let instruction = Instruction::new(
                nearest_goal.x - simulation.racer.x,
                nearest_goal.y - simulation.racer.y,
            );

            let result = simulation.tick(instruction);

            if (result & TickFlag::COLLIDED) != 0 {
                collided_count += 1;
            }

            if (result & TickFlag::GOAL_REACHED) != 0 {
                println!("We collected another checkpoint after {} ticks!", tick);
                println!("Number of collisions on the way: {}", collided_count);
                println!("Checkpoints obtained: {:?}", simulation.reached_goals);
                println!();
                break;
            }
            tick += 1;
        }
    }
    println!("Race completed!");
}
