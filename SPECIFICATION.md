# Specification
The following document contains a specification of the Asteracer simulation, should you wish to implement one in the language of your choice.

## Datatypes and objects
To make sure Asteracer runs in the same way in different languages, **all computations are done strictly using integer datatypes,** which are the following:

- instruction values `InstType: int8`
- position values `PosType: int32`
- speed values `SpeedType: int32`
- size values `SizeType: uint32`

These are then used to define the following objects which we use throughout the simulation:

```python3
class Racer:
    x:  PosType = 0
    y:  PosType = 0
    vx: SpeedType = 0
    vy: SpeedType = 0
    radius: SizeType = 1
```

```python3
class Asteroid:
    x: PosType = 0
    y: PosType = 0
    radius: SizeType = 1
```

```python3
class Goal:
    x: PosType = 0
    y: PosType = 0
    radius: SizeType = 1
```

```python3
class Instruction:
    vx: InstType = 0
    vy: InstType = 0
```

```python3
class BoundingBox:
    min_x: PosType
    min_y: PosType
    max_x: PosType
    max_y: PosType
```

## Simulation
Each tick of the simulation can be separated into the following three steps:

1. **move the racer** using the specified instruction
2. **resolve collisions**
3. **check** if any **goal** is intersected (marking them as such)

These are detailed in the upcoming sections.

_Note 1: for the purpose of optimization, it's advisable to use some kind of a space-partitioning datastructure (like a grid or a K-d tree) for storing asteroids and goals so only a small amount of intersections needs to be checked._

_Note 2: the integer square root the simulation uses is Python's `math.isqrt`, which is "the floor of the exact square root of n, or equivalently the greatest integer a such that a² ≤ n."_

### 1) Moving the racer
Given an instruction `(vx, vy)`, the racer is moved via the following rules:
- slow the racer down by 10%: `racer.velocity = (racer.velocity * 9) // 10`
- add the instruction to the racer's velocity: `racer.velocity += (vx, vy)`
- move the racer using its velocity: `racer.position += racer.velocity`

### 2) Resolving collisions
Collisions are resolved first with asteroids, second with the bounding box.

If any collisions occur, repeat the entire resolution again, at most `5` times in total.

#### Asteroids
Iterate over all asteroids `asteroid` in the order they were added to the simulation and for each:
- let `distance = eucliean_distance(asteroid, racer)` (make sure to use _integer square root_)
- if `distance > asteroid.radius + racer.radius`, we're not colliding (continue to the next asteroid)
- get the vector to push the racer out by: `vn = racer.position - asteroid.position`
- get how much to push by: `push_by = distance - (asteroid.radius + racer.radius)`
- perform the push: `racer.position -= (push_by * vn) / distance`
- penalize by slowing racer down by 50%: `racer.velocity //= 2`

#### Bounding box
For each side of the box, if we're colliding, push the racer out and slow down by 50%.

### 3) Checking goals
Iterate over all goals `goal`, marking them reached if `eucliean_distance(racer, goal) < racer.radius + goal.radius` (i.e. if we're intersecting it).