"""A sample solution to the test map using Asteracer's default implementation."""
from asteracer import *

simulation = Simulation.load(f"../../maps/test.txt")

print(f"Starting racer position: {simulation.racer.x} {simulation.racer.y}")
print(f"Number of asteroids: {len(simulation.asteroids)}")
print(f"Number of goals: {len(simulation.goals)}")
print()

# fly to the right until we hit the wall
tick = 0
print("Flying to the right...")
while True:
    result = simulation.tick(Instruction.right())

    if result & TickFlag.COLLIDED:
        print(f"We collided after {tick} ticks! Ouch...")
        print(f"Current racer position: {simulation.racer.x} {simulation.racer.y}")
        print()
        break

    tick += 1

# if we now fly down, there is a checkpoint that we can collect
print("Flying down...")
while True:
    result = simulation.tick(Instruction.down())

    if result & TickFlag.GOAL_REACHED:
        print(f"We collected a checkpoint after {tick} ticks!")
        print(f"Checkpoints obtained: {simulation.reached_goals}")
        print(f"Current racer position: {simulation.racer.x} {simulation.racer.y}")
        print()
        break

    tick += 1

# collect all goals by always flying to the nearest one
for _ in range(simulation.reached_goals.count(False)):
    # find the nearest goal
    nearest_goal = None
    nearest_goal_distance = float('inf')

    for i, reached in enumerate(simulation.reached_goals):
        if not reached:
            goal = simulation.goals[i]
            distance = euclidean_distance(goal.x, goal.y, simulation.racer.x, simulation.racer.y)

            if distance < nearest_goal_distance:
                nearest_goal_distance = distance
                nearest_goal = goal

    print("Flying to the nearest goal in a straight line...")
    collided_count = 0
    while True:
        instruction = Instruction(
            nearest_goal.x - simulation.racer.x,
            nearest_goal.y - simulation.racer.y,
        )

        result = simulation.tick(instruction)

        if result & TickFlag.COLLIDED:
            collided_count += 1

        if result & TickFlag.GOAL_REACHED:
            print(f"We collected another checkpoint after {tick} ticks!")
            print(f"Number of collisions on the way: {collided_count}")
            print(f"Checkpoints obtained: {simulation.reached_goals}")
            print()
            break

        tick += 1

print(f"Race completed!")
