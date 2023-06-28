"""Asteracer map generator. Use either as module or by calling directly, in which case all maps are generated."""
import os.path
from enum import Enum
from random import seed, randint

import drawsvg as draw
from noise import snoise2

from asteracer import *

ASTEROID_R = 30000
RACER_R = 1000


class SimulationType(Enum):
    TEST = 0
    SPRINT = 1
    MARATHON = 2


def _get_asteroid_probability(x: float, y: float, octaves: int = 1, frequency: float = 2 ** 17) -> float:
    """Return the probability that an asteroid is at those coordinates (using Perlin noise)."""
    return (snoise2(x / frequency, y / frequency, octaves) + 1) / 2


def _collides(o1: Union[Asteroid, Goal], objs: List[Union[Asteroid, Goal]]) -> bool:
    """Return True if an object collides with any of the other objects."""
    for o2 in objs:
        if np.linalg.norm([o1.x - o2.x, o1.y - o2.y]) <= o1.radius + o2.radius:
            return True
    return False


def generate_simulation(simulation_type: SimulationType = SimulationType.SPRINT) -> Simulation:
    """Generate one of the default simulations."""
    goal_count = 0
    safe_zone = RACER_R * 100

    if simulation_type == SimulationType.SPRINT:
        seed(0xC0FFEE5)
        asteroid_count = 600
        max_x = max_y = 500000
        min_x = min_y = -max_x
        center = (min_x + safe_zone // 2, min_y + safe_zone // 2)

    elif simulation_type == SimulationType.MARATHON:
        safe_zone *= 2
        seed(0xBEEF)
        asteroid_count = 1500
        goal_count = 70
        max_x = max_y = 1200000
        min_x = min_y = -max_x
        center = (0, 0)

    else:
        seed(0x123456789)
        safe_zone /= 10
        asteroid_count = 10
        goal_count = 4
        max_x = max_y = 100000
        min_x = min_y = -max_x
        center = (0, 0)

    asteroids = []
    goals = []

    k = 10

    def get_asteroid_or_goal():
        max_dist = 0
        max_pos = None

        for _ in range(k):
            # no spawn camping
            while True:
                x = randint(-max_x, max_x)
                y = randint(-max_y, max_y)

                p = _get_asteroid_probability(x, y)

                if p < 0.25:
                    continue

                # for sprint, also make the other side clear (goal is there)
                if simulation_type == SimulationType.SPRINT:
                    if -center[0] - safe_zone // 2 <= x <= -center[1] + safe_zone // 2 \
                            and -center[1] - safe_zone // 2 <= y <= -center[1] + safe_zone // 2:
                        continue

                # if we're in the safe zone, try again
                if center[0] - safe_zone // 2 <= x <= center[1] + safe_zone // 2 \
                        and center[1] - safe_zone // 2 <= y <= center[1] + safe_zone // 2:
                    continue

                break

            curr_dist = float('inf')
            for asteroid in asteroids:
                curr_dist = min(curr_dist, np.linalg.norm((asteroid.x - x, asteroid.y - y)))

            if curr_dist > max_dist:
                max_dist = curr_dist
                max_pos = (x, y)

        x, y = max_pos

        return Asteroid(x=PosType(x), y=PosType(y), radius=SizeType(int(ASTEROID_R * _get_asteroid_probability(x, y))))

    for _ in range(asteroid_count):
        asteroids.append(get_asteroid_or_goal())

    if simulation_type == SimulationType.SPRINT:
        goals.append(Asteroid(x=PosType(-center[0]), y=PosType(-center[1]), radius=SizeType(ASTEROID_R // 2)))
    else:
        i = 0
        while i < goal_count:
            g = get_asteroid_or_goal()

            if _collides(g, asteroids + goals):
                continue

            goals.append(g)
            i += 1

    return Simulation(
        racer=Racer(x=PosType(center[0]), y=PosType(center[1]), radius=SizeType(RACER_R)),
        bounding_box=BoundingBox(
            min_x=PosType(min_x), min_y=PosType(min_y),
            max_x=PosType(max_x), max_y=PosType(max_y)
        ),
        asteroids=asteroids,
        goals=goals,
    )


def get_preview(simulation: Simulation, size=1000):
    """Generate preview of the simulation as an SVG."""
    d = draw.Drawing(size, size, origin='center')

    # background
    d.append(
        draw.Rectangle(
            -size / 2, -size / 2,
            size, size,
            fill="White"
        )
    )

    def draw_circles(circles: List[Union[Asteroid, Goal, Racer]], color: str):
        """Draw a circle with a specific color on the SVG."""
        s = simulation.bounding_box.width() / size

        for i, circle in enumerate(circles):
            d.append(draw.Circle(circle.x / s, circle.y / s, circle.radius / s, fill=color, stroke=color))

    draw_circles(simulation.asteroids, "Black")
    draw_circles(simulation.goals, "Red")
    draw_circles([simulation.racer], "Gray")

    return d


if __name__ == "__main__":
    os.chdir(os.path.abspath(os.path.dirname(__file__)))

    # generate all maps
    for simulation_type in SimulationType:
        print(f"Generating {simulation_type.name.lower()}... ", end="", flush=True)
        simulation = generate_simulation(simulation_type)
        simulation.save(f"../maps/{simulation_type.name.lower()}.txt")
        get_preview(simulation).save_svg(f"../maps/{simulation_type.name.lower()}.svg")
        print("done.", flush=True)
