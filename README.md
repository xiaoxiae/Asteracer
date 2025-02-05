<img align="left" width="200" src="logo.png" />

# Asteracer
A TAS-focused space racing game.

Compete for the fastest time to collect all checkpoints while avoiding the asteroids on a wide variety of maps.

## Getting started
For starting out with writing solves, see `src/pyasteracer/example.py`.

## Contents
- `src/` – source code of Asteracer's implementations
  - `pyasteracer/` – Python implementation (as a module)
    - `__init__.py` – the implementation
    - `__main__.py` – a PyQt5 visualizer (_under construction_)
    - `example.py` – a simple solution for the test map
    - `generator.py` – map generator
    - `solver.py` – useful utilities for solving Asteracer
    - `test.py` – scripts for testing against this implementation
  - `cpp/` – C++ implementation (used in Webassembly)
    - `sim.cpp` – the implementation
- `test/` – test data (maps, instructions and simulation states)
- `maps/` – official maps (in `txt` and `svg` form)
- `graphs/` – graphs of maps for use in solving Asteracer

## Maps
There are currently two official maps Asteracer can be played on.

| Test                             | Sprint                             | Marathon                               |
| --- | --- | --- |
| ![Test Preview](maps/test.svg)| ![Sprint Preview](maps/sprint.svg) | ![Marathon Preview](maps/marathon.svg) | 
| _Test map._ | _Smaller, one goal._ | _Larger, multiple goals._ | 

## Game Specification
If you wish to implement Asteracer using a different language (which you're very welcome to do), refer to `SPECIFICATION.md`, which covers the design and specification of Asteracer in great detail.
Feel free to create an issue/let me know so I can a link to it here!
