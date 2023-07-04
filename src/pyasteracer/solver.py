"""Useful functions for writing an Asteracer solver. When called as a script, generates graphs for all maps."""
import os
from typing import Iterator

import drawsvg as draw
import shapely

from pyasteracer import *
from pyasteracer.generator import RACER_R, ASTEROID_R, get_preview, SimulationType

Point = Tuple[float, float]
Vertex = Tuple[int, int]
Edge = Tuple[Vertex, Vertex]


def yield_points_at_distance(x: float, y: float, r: float, n: int) -> Iterator[Point]:
    """Generate n points uniformly at distance r from the coordinates (x, y)."""
    for i in range(n):
        t = (i / n) * np.pi * 2
        yield x + np.cos(t) * r, y + np.sin(t) * r


def circle_segment_intersection(p1: Point, p2: Point, C: Point, r: float) -> bool:
    """Return True if the circle (C, r) intersects with a line segment (p1, p2)."""
    return shapely.intersects(shapely.Point(*C).buffer(r), shapely.LineString([p1, p2]))


def is_point_in_asteroid(p: Point, simulation: Simulation) -> bool:
    """Return True if the coordinate is in any of the asteroids."""
    x, y = p
    grid_x, grid_y = simulation._coordinate_to_grid(*p)

    if (grid_x, grid_y) not in simulation._grid:
        return False

    for asteroid in simulation._grid[(grid_x, grid_y)]:
        if np.linalg.norm((asteroid.x - x, asteroid.y - y)) <= asteroid.radius:
            return True
    return False


def is_segment_clear(p1: Point, p2: Point, simulation: Simulation, offset=0) -> bool:
    """Return True if the line doesn't intersect any of the asteroids (+ offset)."""
    grid_x1, grid_y1 = simulation._coordinate_to_grid(*p1)
    grid_x2, grid_y2 = simulation._coordinate_to_grid(*p2)

    for x in range(min(grid_x1, grid_x2), max(grid_x1, grid_x2) + 1):
        for y in range(min(grid_y1, grid_y2), max(grid_y1, grid_y2) + 1):
            for asteroid in simulation._grid[(x, y)]:
                if circle_segment_intersection(p1, p2, (asteroid.x, asteroid.y), asteroid.radius + offset):
                    return False
    return True


def angle_between_points(p1: Point, p2: Point, p3: Point) -> float:
    """Return the (smaller) angle between three points."""
    p1 = np.array(p1)
    p2 = np.array(p2)
    p3 = np.array(p3)

    a = p1 - p2
    b = p3 - p2

    angle = np.clip(np.dot(a, b) / (np.linalg.norm(a) * np.linalg.norm(b)), -1, 1)
    return np.arccos(angle)


def valid_edge_angle(u: Vertex, v: Vertex, asteroid_vertices: Dict[Vertex, Asteroid], max_edge_angle: float) -> bool:
    """Return True if the angles between an edge and its asteroids are valid."""
    for v1, v2 in [(u, v), (v, u)]:
        # if the opposing side isn't an asteroid, don't check the angle
        if v1 not in asteroid_vertices:
            continue

        if angle_between_points((int(asteroid_vertices[v1].x), int(asteroid_vertices[v1].y)), v1, v2) > max_edge_angle:
            return False

    return True


def is_point_in_bounds(bb: BoundingBox, p: Point) -> bool:
    """Return True if the given vertex is in bounds."""
    return bb.min_x <= p[0] <= bb.max_x and bb.min_x <= p[1] <= bb.max_x


def get_asteroid_graph(
        simulation: Simulation,
        asteroid_keypoint_rate=1 / ASTEROID_R * 20 ** 2,
        goal_keypoint_rate=1 / ASTEROID_R * 12 ** 2,
        asteroid_keypoint_offset=RACER_R * 1.75,
        goal_keypoint_offset=RACER_R * 0.25,
        max_edge_length=ASTEROID_R * 10,
        max_edge_angle=(3 / 5) * np.pi,
) -> Tuple[List[Vertex], List[Edge], Dict[Vertex, Asteroid], Dict[Vertex, Goal]]:
    """Return a graph with vertices being keypoints and edges their line of sight.

    :param simulation: the simulation to generate the graph of
    :param asteroid_keypoint_rate: the rate of keypoints generated for each asteroid
    :param goal_keypoint_rate: the rate of keypoints generated for each goal
    :param asteroid_keypoint_offset: offset of the keypoints from asteroids
    :param goal_keypoint_offset: offset of the keypoints from goals
    :param max_edge_length: maximum length of the edge in the graph
    :param max_edge_angle: maximum asteroid-edge angle
    :return: a graph of the simulation, along with dictionaries for mapping vertices to asteroids/goals
    """
    vertices: List[Vertex] = [(simulation.racer.x, simulation.racer.y)]
    edges: List[Edge] = []

    asteroid_vertices: Dict[Vertex, Asteroid] = {}
    goal_vertices: Dict[Vertex, Goal] = {}

    # generate asteroid keypoints
    for asteroid in simulation.asteroids:
        for keypoint in yield_points_at_distance(
                asteroid.x, asteroid.y,
                asteroid.radius + asteroid_keypoint_offset,
                round(np.sqrt(asteroid.radius * asteroid_keypoint_rate)),
        ):
            if is_point_in_asteroid(keypoint, simulation) or not is_point_in_bounds(simulation.bounding_box, keypoint):
                continue

            vertices.append((round(keypoint[0]), round(keypoint[1])))
            asteroid_vertices[vertices[-1]] = asteroid

    # generate goal keypoints (vertices)
    for goal in simulation.goals:
        for keypoint in yield_points_at_distance(
                goal.x, goal.y,
                goal.radius + goal_keypoint_offset,
                round(np.sqrt(goal.radius * goal_keypoint_rate)),
        ):
            if is_point_in_asteroid(keypoint, simulation) or not is_point_in_bounds(simulation.bounding_box, keypoint):
                continue

            vertices.append((round(keypoint[0]), round(keypoint[1])))
            goal_vertices[vertices[-1]] = goal

    # generate edges
    for i in range(len(vertices)):
        for j in range(i + 1, len(vertices)):
            v1 = np.array(vertices[i])
            v2 = np.array(vertices[j])

            d = np.linalg.norm(v1 - v2)

            if d > max_edge_length:
                continue

            if not valid_edge_angle(vertices[i], vertices[j], asteroid_vertices, max_edge_angle):
                continue

            if not is_segment_clear(vertices[i], vertices[j], simulation, simulation.racer.radius):
                continue

            edges.append((vertices[i], vertices[j]))

    # also generate vertices for centers of goals with edges to the other edges of the goal
    # this will likely be useful for a number of approaches
    for goal in simulation.goals:
        u = (goal.x, goal.y)
        vertices.append(u)
        goal_vertices[u] = goal

        for v in vertices:
            if v in goal_vertices and goal_vertices[v] is goal:
                edges.append((u, v))

    return vertices, edges, asteroid_vertices, goal_vertices


def save_asteroid_graph(
        path: str,
        vertices,
        edges,
        asteroid_vertices,
        goal_vertices,
):
    """Save the asteroid graph to a file."""
    with open(path, "w") as f:
        # f.write(f"# Generated with the following parameters:\n")
        # f.write(f"# ----------------------------------------\n")

        # for k, v in signature.parameters.items():
        #    if v.default is inspect.Parameter.empty:
        #        continue

        #    f.write(f"# {k}: {v.default}\n")
        # f.write(f"# ----------------------------------------\n\n")

        f.write(f"{len(vertices)} {len(edges)}\n")

        for vertex in vertices:
            f.write(f"{vertex[0]} {vertex[1]}")

            if vertex in asteroid_vertices:
                f.write(f" A {asteroid_vertices[vertex].x} {asteroid_vertices[vertex].y}\n")
            elif vertex in goal_vertices:
                f.write(f" G {goal_vertices[vertex].x} {goal_vertices[vertex].y}\n")
            else:
                f.write(f" S\n")

        for (x1, y1), (x2, y2) in edges:
            f.write(f"{x1} {y1} {x2} {y2}\n")


def load_asteroid_graph(path: str, simulation: Simulation):
    """Load the asteroid graph from a file."""
    with open(path) as f:
        contents = [line for line in f.read().splitlines() if not line.startswith("#") and line.strip() != ""]

        n, m = list(map(int, contents[0].split()))

        vertices = []
        edges = []

        asteroid_vertices = {}
        goal_vertices = {}

        for i in range(1, 1 + n):
            line = contents[i].split()

            vertices.append((int(line[0]), int(line[1])))

            if line[2] == "S":
                continue

            obj_pos = (int(line[3]), int(line[4]))

            if line[2] == "A":
                for asteroid in simulation._grid[simulation._coordinate_to_grid(*obj_pos)]:
                    if asteroid.x == obj_pos[0] and asteroid.y == obj_pos[1]:
                        asteroid_vertices[vertices[-1]] = asteroid
                        break
                else:
                    raise Exception(f"Unmatched asteroid vertex (line {i}).")
            elif line[2] == "G":
                for goal in simulation.goals:
                    if goal.x == obj_pos[0] and goal.y == obj_pos[1]:
                        goal_vertices[vertices[-1]] = goal
                        break
                else:
                    raise Exception(f"Unmatched goal vertex (line {i}).")

        for i in range(1 + n, 1 + n + m):
            line = contents[i].split()

            edges.append(
                (
                    (int(line[0]), int(line[1])),
                    (int(line[2]), int(line[3])),
                )
            )

    return vertices, edges, asteroid_vertices, goal_vertices


def get_graph_preview(
        simulation: Simulation,
        vertices: List[Vertex],
        edges: List[Edge],
) -> draw.Drawing:
    """Return an SVG object with the simulation graph."""
    d = get_preview(simulation)
    s = simulation.bounding_box.width() / d.width

    for vertex in vertices:
        x, y = vertex
        d.append(draw.Circle(x / s, y / s, simulation.racer.radius / s, fill="Gray"))

    for u, v in edges:
        x1, y1 = u
        x2, y2 = v
        d.append(draw.Line(x1 / s, y1 / s, x2 / s, y2 / s, stroke="Gray", opacity='0.35'))

    return d


if __name__ == "__main__":
    os.chdir(os.path.abspath(os.path.dirname(__file__)))

    # generate all graphs
    for simulation_type in SimulationType:
        simulation = Simulation.load(f"../../maps/{simulation_type.name.lower()}.txt")
        task = simulation_type.name.lower()

        print(f"Generating {task} graph... ", end="", flush=True)
        vertices, edges, asteroid_vertices, goal_vertices = get_asteroid_graph(simulation)
        print("saving... ", end="", flush=True)
        save_asteroid_graph(f"../../graphs/{task}.txt", vertices, edges, asteroid_vertices, goal_vertices)
        print("generating preview... ", end="", flush=True)
        d = get_graph_preview(simulation, vertices, edges)
        d.save_svg(f"../../graphs/{task}.svg")
        print("done.", flush=True)
