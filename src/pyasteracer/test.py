"""A script for testing Asteracer."""
import argparse
import random
import sys
import glob

from pyasteracer import *


def save_states(
    simulation: Simulation,
    instructions: List[Instruction],
    path: str,
):
    """Output states of the simulation given instructions."""
    r = simulation.racer

    with open(path, "w") as f:
        for i, instruction in enumerate(instructions):
            simulation.tick(instructions[i])
            f.write(f"{r.x} {r.y} {r.vx} {r.vy} {''.join([str(int(t)) for t in simulation.reached_goals])}\n")


def verify_states(
    simulation: Simulation,
    instructions: List[Instruction],
    path: str,
):
    """Verify the states of the simulation given instructions and states path to check against."""
    r = simulation.racer

    with open(path, "r") as f:
        lines = f.readlines()

        for i, instruction in enumerate(instructions):
            simulation.tick(instructions[i])

            parts = lines[i].split()

            assert r.x == int(parts[0]) and r.y == int(parts[1]), f"Incorrect Racer position after instruction {i}!"
            assert r.vx == int(parts[2]) and r.vy == int(parts[3]), f"Incorrect Racer velocity after instruction {i}!"
            assert simulation.reached_goals == [c == '1' for c in parts[4]], f"Incorrect goal states after instruction {i}!"


def run(arguments=None):
    if arguments:
        tests = [(
            arguments.simulation_path, arguments.instructions_path, arguments.states_path, arguments.mode
        )]
    else:
        tests = [
                (file, file[:-3] + "in", file[:-3] + "out", "verify")
            for file in glob.glob("../test/*.txt")
        ]

    for simulation_path, instructions_path, states_path, mode in tests:
        simulation = Simulation.load(simulation_path)

        if mode == "verify":
            print(f"Verifying '{simulation_path}'!")
            instructions = load_instructions(instructions_path)
            verify_states(simulation, instructions, states_path)
        else:
            print(f"Generating '{simulation_path}'!")

            instructions = [
                Instruction.random()
                for _ in range(arguments.n)
            ]

            save_instructions(arguments.instructions_path, instructions)
            save_states(simulation, instructions, states_path)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="A script for testing the Asteracer Python implementation.")

    parser.add_argument("--simulation-path", required=True, help="The path to the simulation file.")
    parser.add_argument("--instructions-path", required=True, help="The path to a file with the instructions.")
    parser.add_argument("--states-path", required=True, help="The path to a file with the simulation states.")

    parser.add_argument("--mode", required=True, help="Mode to run.", choices=["verify", "generate"])
    parser.add_argument("--n", help="Number of instructions to generate. Only used for mode='generate'.")

    # if ran without arguments, obtain them from the test directory
    run(parser.parse_args() if len(sys.argv) != 1 else None)

