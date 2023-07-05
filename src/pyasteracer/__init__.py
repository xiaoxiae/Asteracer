"""The Asteracer game implementation. Includes the base movement code + couple of QOL additions (eg. save states)."""
from __future__ import annotations

import dataclasses
from collections import defaultdict
from dataclasses import dataclass
from math import isqrt
from typing import List, Union, Tuple, Dict

import numpy as np

InstType = np.int8
PosType = np.int64
SpeedType = np.int64
SizeType = np.int64


class TickFlag:
    """Flags returned by simulation.tick() for various events that can occur during a tick."""
    COLLIDED = 1
    GOAL_REACHED = 2


@dataclass
class Racer:
    x: PosType = 0
    y: PosType = 0
    vx: SpeedType = 0
    vy: SpeedType = 0
    radius: SizeType = 1


@dataclass(frozen=True)
class Asteroid:
    x: PosType = 0
    y: PosType = 0
    radius: SizeType = 1


Goal = Asteroid


class Instruction:
    MAX_ACCELERATION = 127

    def __init__(self, vx: Union[int, float] = 0, vy: Union[int, float] = 0):
        """Whatever values we get, normalize them."""
        
        if distance_squared(vx, vy) > Instruction.MAX_ACCELERATION ** 2:
            distance = euclidean_distance(vx, vy)
            vx = np.clip((vx * Instruction.MAX_ACCELERATION) // distance, -Instruction.MAX_ACCELERATION, Instruction.MAX_ACCELERATION)
            vy = np.clip((vy * Instruction.MAX_ACCELERATION) // distance, -Instruction.MAX_ACCELERATION, Instruction.MAX_ACCELERATION)

        self.vx = InstType(vx)
        self.vy = InstType(vy)

    def __hash__(self):
        return hash((self.vx, self.vy))

    def __eq__(self, other):
        return self.vx == other.vx and self.vy == other.vy

    def __str__(self):
        return f"Instruction({self.vx}, {self.vy})"

    @classmethod
    def up(cls):
        return cls(0, np.iinfo(InstType).min)

    @classmethod
    def down(cls):
        return cls(0, np.iinfo(InstType).max)

    @classmethod
    def left(cls):
        return cls(np.iinfo(InstType).min, 0)

    @classmethod
    def right(cls):
        return cls(np.iinfo(InstType).max, 0)


@dataclass
class BoundingBox:
    min_x: PosType
    min_y: PosType
    max_x: PosType
    max_y: PosType

    def width(self) -> SizeType:
        return SizeType(self.max_x - self.min_x)

    def height(self) -> SizeType:
        return SizeType(self.max_y - self.min_y)


def distance_squared(x1, y1, x2=0, y2=0) -> PosType:
    """Squared Euclidean distance between two points."""
    return PosType((x1 - x2) ** 2 + (y1 - y2) ** 2)


def euclidean_distance(x1, y1, x2=0, y2=0):
    """Integer Euclidean distance between two points. Uses integer square root."""
    # TODO: provide custom and simple implementation of isqrt
    return PosType(isqrt(distance_squared(x1, y1, x2, y2)))


class Simulation:
    DRAG_FRACTION = (9, 10)  # slowdown of the racer's velocity after each tick
    COLLISION_FRACTION = (1, 2)  # slowdown of the racer's velocity after a tick where a collision occurred
    MAX_COLLISION_RESOLUTIONS = 5  # at most how many collision iterations to perform

    def __init__(
            self,
            racer: Racer = Racer(),
            asteroids: List[Asteroid] = None,
            goals: List[Goal] = None,
            bounding_box: BoundingBox = None,
    ):
        # the initial racer state (used when resetting the simulation)
        self.initial_racer = dataclasses.replace(racer)

        self.racer = racer
        self.asteroids = asteroids or []
        self.goals = goals or []
        self.bounding_box = bounding_box

        # to speed up the computation, we divide the bounding box (if we have one) into a grid
        # we do this so we don't need to check all asteroids at each tick, only those that could collide with the racer
        self._grid_size = len(self.asteroids) // 10 or 1
        self._grid: Dict[Tuple[int, int], List[Asteroid]] = defaultdict(list)

        for asteroid in asteroids:
            min_x, min_y = self._coordinate_to_grid(
                asteroid.x - asteroid.radius - racer.radius,
                asteroid.y - asteroid.radius - racer.radius,
                )

            max_x, max_y = self._coordinate_to_grid(
                asteroid.x + asteroid.radius + racer.radius,
                asteroid.y + asteroid.radius + racer.radius,
                )

            for grid_x in range(min_x, max_x + 1):
                for grid_y in range(min_y, max_y + 1):
                    self._grid[(grid_x, grid_y)].append(asteroid)

        self.reached_goals: List[bool] = [False] * len(self.goals)

        # a list of simulation states that can be popped (restored to)
        self._pushed_states = []

    def _coordinate_to_grid(self, x: float, y: float) -> Tuple[int, int]:
        """Translate an (x,y) coordinate into a coordinate of the grid."""
        if self.bounding_box is None:
            return 0, 0

        return (
            int((x - self.bounding_box.min_x) / self.bounding_box.width() * self._grid_size),
            int((y - self.bounding_box.min_y) / self.bounding_box.height() * self._grid_size),
        )

    def _move_racer(self, instruction: Instruction):
        """Move the racer in the given direction."""
        vx, vy = instruction.vx, instruction.vy

        # drag
        self.racer.vx = self.racer.vx * self.DRAG_FRACTION[0] // self.DRAG_FRACTION[1]
        self.racer.vy = self.racer.vy * self.DRAG_FRACTION[0] // self.DRAG_FRACTION[1]

        # velocity
        self.racer.vx += vx
        self.racer.vy += vy

        # movement
        self.racer.x += self.racer.vx
        self.racer.y += self.racer.vy

    def _push_out(self, obj: Union[Asteroid, BoundingBox]) -> bool:
        """Attempt to push the racer out of the object (if he's colliding), adjusting
        his velocity accordingly (based on the angle of collision). Returns True if the
        racer was pushed out, otherwise returns False."""
        if isinstance(obj, Asteroid):
            # not colliding, nothing to be done
            if distance_squared(self.racer.x, self.racer.y, obj.x, obj.y) > (self.racer.radius + obj.radius) ** 2:
                return False

            # the vector to push the racer out by
            nx = self.racer.x - obj.x
            ny = self.racer.y - obj.y

            # how much to push by
            distance = euclidean_distance(self.racer.x, self.racer.y, obj.x, obj.y)
            push_by = distance - (self.racer.radius + obj.radius)

            # the actual push
            self.racer.x -= (nx * push_by) // distance
            self.racer.y -= (ny * push_by) // distance

            return True

        elif isinstance(obj, BoundingBox):
            # not pretty but easy to read :)
            collided = False

            if self.racer.x - self.racer.radius < obj.min_x:
                self.racer.x = obj.min_x + self.racer.radius
                collided = True
            if self.racer.x + self.racer.radius > obj.max_x:
                self.racer.x = obj.max_x - self.racer.radius
                collided = True
            if self.racer.y - self.racer.radius < obj.min_y:
                self.racer.y = obj.min_y + self.racer.radius
                collided = True
            if self.racer.y + self.racer.radius > obj.max_y:
                self.racer.y = obj.max_y - self.racer.radius
                collided = True

            return collided

        else:
            raise Exception("Attempted to collide with something other than asteroid / bounding box!")

    def _check_goal(self) -> bool:
        """Sets the _reached_goals variable to True according to if the racer is intersecting them, returning True if
        a new one was reached."""
        new_goal_reached = False

        for i, goal in enumerate(self.goals):
            if distance_squared(self.racer.x, self.racer.y, goal.x, goal.y) <= (self.racer.radius + goal.radius) ** 2:
                if not self.reached_goals[i]:
                    new_goal_reached = True

                self.reached_goals[i] = True

        return new_goal_reached

    def _resolve_collisions(self) -> bool:
        """Resolve all collisions of the racer and asteroids, returning True if a collison occurred."""
        collided = False
        for _ in range(self.MAX_COLLISION_RESOLUTIONS):
            collided_this_iteration = False

            for asteroid in self._grid[self._coordinate_to_grid(self.racer.x, self.racer.y)]:
                if self._push_out(asteroid):
                    collided_this_iteration = collided = True
                    break

            if self.bounding_box is not None and self._push_out(self.bounding_box):
                collided_this_iteration = collided = True

            if not collided_this_iteration:
                break

        if collided:
            self.racer.vx = self.racer.vx * self.COLLISION_FRACTION[0] // self.COLLISION_FRACTION[1]
            self.racer.vy = self.racer.vy * self.COLLISION_FRACTION[0] // self.COLLISION_FRACTION[1]

        return collided

    def finished(self) -> bool:
        """Returns True if the racer reached all goals."""
        return all(self.reached_goals)

    def restart(self):
        """Restart the simulation to its initial state."""
        self.racer.x = self.initial_racer.x
        self.racer.y = self.initial_racer.y
        self.racer.vx = 0
        self.racer.vy = 0

        for i in range(len(self.reached_goals)):
            self.reached_goals[i] = False

    def tick(self, instruction: Instruction):
        """Simulate a single tick of the simulation."""
        self._move_racer(instruction)
        collided = self._resolve_collisions()
        goal = self._check_goal()

        return (TickFlag.COLLIDED if collided else 0) | (TickFlag.GOAL_REACHED if goal else 0)

    def simulate(self, instructions: List[Instruction]):
        """Simulate a number of instructions for the simulation (from the start)."""
        self.restart()

        for instruction in instructions:
            self.tick(instruction)

    def save(self, path: str):
        """Save the simulation to a file:
        | 0 0 5              // starting racer x/y/radius
        | -100 -100 100 100  // bounding box (min_x/min_y/max_x/max_y)
        | 5                  // number of asteroids
        | 10 -10 10          // asteroid 1 x/y/radius
        | 20 20 50           // asteroid 2 x/y/radius
        | -10 10 30          // asteroid 3 x/y/radius
        | 10 10 70           // asteroid 4 x/y/radius
        | -10 -10 10         // asteroid 5 x/y/radius
        | 1                  // number of goals
        | 100 100 10         // goal 1 x/y/radius
        """
        with open(path, "w") as f:
            f.write(f"{self.racer.x} {self.racer.y} {self.racer.radius}\n")

            bbox = self.bounding_box
            f.write(f"{bbox.min_x} {bbox.min_y} {bbox.max_x} {bbox.max_y}\n")

            f.write(f"{len(self.asteroids)}\n")
            for asteroid in self.asteroids:
                f.write(f"{asteroid.x} {asteroid.y} {asteroid.radius}\n")

            f.write(f"{len(self.goals)}\n")
            for goal in self.goals:
                f.write(f"{goal.x} {goal.y} {goal.radius}\n")

    @classmethod
    def load(cls, path: str) -> Simulation:
        """Load the simulation from a file (see self.save for the format description)."""
        with open(path) as f:
            lines = f.read().splitlines()

        racer_parts = lines[0].split()
        racer = Racer(x=PosType(racer_parts[0]), y=PosType(racer_parts[1]), radius=SizeType(racer_parts[2]))

        bb_parts = lines[1].split()
        bb = BoundingBox(PosType(bb_parts[0]), PosType(bb_parts[1]), PosType(bb_parts[2]), PosType(bb_parts[2]))

        asteroid_count = int(lines[2])

        asteroids = []
        for i in range(3, 3 + asteroid_count):
            asteroid_parts = lines[i].split()
            asteroids.append(
                Asteroid(
                    x=PosType(asteroid_parts[0]),
                    y=PosType(asteroid_parts[1]),
                    radius=SizeType(asteroid_parts[2]),
                )
            )

        goal_count = int(lines[3 + asteroid_count])

        goals = []
        for i in range(4 + asteroid_count, 4 + asteroid_count + goal_count):
            goal_parts = lines[i].split()
            goals.append(
                Asteroid(
                    x=PosType(goal_parts[0]),
                    y=PosType(goal_parts[1]),
                    radius=SizeType(goal_parts[2]),
                )
            )

        return Simulation(racer=racer, bounding_box=bb, asteroids=asteroids, goals=goals)

    def push(self):
        """Push (save) the current state of the simulation. Can be popped (restored) later."""
        self._pushed_states.append(
            (
                dataclasses.replace(self.racer),
                list(self.reached_goals),
            )
        )

    def pop(self):
        """Pop (restore) the previously pushed state."""
        assert len(self._pushed_states) != 0, "No states to pop!"
        self.racer, self.reached_goals = self._pushed_states.pop()

    def apply(self):
        """Apply the previously pushed state without popping it."""
        self.racer = dataclasses.replace(self._pushed_states[-1][0])
        self.reached_goals = list(self._pushed_states[-1][1])


def save_instructions(path: str, instructions: List[Instruction]):
    """Save a list of instructions to a file:
    | 4         // number if instructions
    | -16 -127  // instructions...
    | -16 -127
    | -26 -125
    | -30 -124
    """
    with open(path, "w") as f:
        f.write(f"{len(instructions)}\n")

        for instruction in instructions:
            f.write(f"{instruction.vx} {instruction.vy}\n")


def load_instructions(path: str) -> List[Instruction]:
    """Load a list of instructions from a file (see save_instructions for the format description)."""
    instructions = []

    with open(path) as f:
        for line in f.read().splitlines()[1:]:
            instruction_parts = list(map(InstType, line.split()))
            instructions.append(Instruction(*instruction_parts))

    return instructions
