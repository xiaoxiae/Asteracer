"""A script for testing Asteracer (mostly against different implementations)."""
import argparse

from asteracer import *


def print_states(simulation: Simulation, instructions: List[Instruction]):
    """Print the states of the racers given """
    r = simulation.racer
    print(f"{r.x} {r.y} {r.vx} {r.vy}")

    for i, instruction in enumerate(instructions):
        simulation.tick(instructions[i])
        print(f"x={r.x} y={r.y} vx={r.vx} vy={r.vy}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="A script for printing out the states of a racer given instructions.")

    parser.add_argument("--simulation-path", required=True, help="The path to the simulation file.")
    parser.add_argument("--instructions-path", required=True, help="The path to a file with the instructions.")

    arguments = parser.parse_args()

    simulation = Simulation.load(arguments.simulation_path)
    instructions = load_instructions(arguments.instructions_path)

    print("Printing the states of the Racer:")
    print_states(simulation, instructions)
