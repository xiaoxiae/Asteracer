# Specification
The following document contains a specification of the Asteracer simulation, should you wish to implement one in the language of your choice.

## Datatypes and objects
To make sure Asteracer runs in the same way in different languages, **all computations are done strictly using integer datatypes,** which are the following:

- instruction values `InstType: int8`
- position values `PosType: int64`
- speed values `SpeedType: int64`
- size values `SizeType: int64`

These are then used to define the following objects which we use throughout the simulation:

```python3
class Racer:
    x:  PosType
    y:  PosType
    vx: SpeedType
    vy: SpeedType
    radius: SizeType
```

```python3
class Asteroid:
    x: PosType
    y: PosType
    radius: SizeType
```

```python3
class Goal:
    x: PosType
    y: PosType
    radius: SizeType
```

```python3
class Instruction:
    vx: InstType
    vy: InstType
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

_Note 1: for the purpose of optimization, it's advisable to use some kind of a space-partitioning datastructure (like a grid or a K-d tree) for storing asteroids and goals so only a small amount of intersections needs to be checked. Remember to take racer's radius into account._

_Note 2: the integer square root the simulation uses is Python's `math.isqrt`, which is "the floor of the exact square root of n, or equivalently the greatest integer a such that a² ≤ n." Note that python's implementation is exact and does not use floating-point sqrt._

### 1) Moving the racer
Given an instruction `(vx, vy)`, the racer is moved via the following rules:
- slow the racer down by 10%: `racer.velocity = (racer.velocity * 9) // 10`
- add the instruction to the racer's velocity: `racer.velocity += (vx, vy)`
- move the racer using its velocity: `racer.position += racer.velocity`

Note that the **division operator** works by **truncating the floating value of the number**.
This especially means that Python's division doesn't work for negative numbers, as `-5 // 2 = -3`, which is, for our purposes, incorrect.
One way to overcome this is to do `abs(x) // v * signum(x)`, which produces the correct result.

An instruction is valid only if the **length of the instruction** (euclidean distance) is **no greater than `127`**.
Note that the instruction length check must be done by comparing the squares of the instruction length and the maximal acceleration (i.e. `distance_squared(vx, vy) > 127 ** 2`), as this can be done exactly, unlike comparing euclidean length computed with `isqrt`.

### 2) Resolving collisions

Collisions are resolved in `1` to `5` subticks.
A subtick check collisions first with all asteroids, then with the bounding box.
If any collision occurs during a subtick, skip all other checks in the current subtick and immediately execute another subtick, until the subtick limit is reached.

If any collision occurs in the tick as a whole, reduce the racer's velocity to half.
This slowdown must occur at most once per tick, no matter how many collisions happened.

#### Asteroids
Iterate over all asteroids `asteroid` in the order they were added to the simulation and for each:
- if `euclidean_distance(asteroid, racer) > (asteroid.radius + racer.radius)`, we're not colliding (continue to the next asteroid)
- execute the following only upon collision:
    - let `distance = euclidean_distance(asteroid, racer)` (make sure to use _integer square root_)
    - get the vector to push the racer out by: `vn = racer.position - asteroid.position`
    - get how much to push by: `push_by = distance - (asteroid.radius + racer.radius)`
    - perform the push: `racer.position -= (push_by * vn) / distance`
    - **break** the asteroid iteration loop (we only allow one asteroid collision per subtick)

#### Bounding box
For each side of the box, if we're colliding (eg. if `racer.x - racer.radius < box.min_x` for the left side), push the racer out to the bounding box edge.

Note that bounding box collisions can occur during a subtick independent of whether an asteroid collision occured.

### 3) Checking goals
Iterate over all goals `goal`, marking them reached if `euclidean_distance(racer, goal) < (racer.radius + goal.radius)` (i.e. if we're intersecting it).
