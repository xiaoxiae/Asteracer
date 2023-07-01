#include <vector>
#include <limits>

typedef int32_t i32;
typedef int64_t i64;

struct Obj
{
    i32 x;
    i32 y;
    i32 r;
};

class GridLookup
{
    // The maximum number of cells in the larger axis will be this value
    static constexpr i32 CellAxisCountTarget = 128;

    std::vector<std::vector<Obj>> map;

    i32 map_cells_x;
    i32 map_cells_y;
    i32 map_start_x;
    i32 map_start_y;
    i32 map_cell_size;

    /**
     * Converts world space coordinates to cell coordinates.
     * Clamps the result to cell map bounds.
    */
    void to_cell_coords(const i32 x, const i32 y, i32& result_x, i32& result_y) const
    {
        result_x = (x - map_start_x) / map_cell_size;
        result_y = (y - map_start_y) / map_cell_size;

        if(result_x < 0)
            result_x = 0;
        if(result_y < 0)
            result_y = 0;
        if(result_x >= map_cells_x)
            result_x = map_cells_x - 1;
        if(result_y >= map_cells_y)
            result_y = map_cells_y - 1;
    }

public:
    GridLookup(const Obj* objects, const i32 objects_count, const i32 racer_radius)
    {
        i32 aabb_min_x = std::numeric_limits<i32>::max();
        i32 aabb_min_y = std::numeric_limits<i32>::max();
        i32 aabb_max_x = std::numeric_limits<i32>::min();
        i32 aabb_max_y = std::numeric_limits<i32>::min();

        // Compute the aabb from the objects list so that the grid is independent of actual play area
        for(i32 i = 0; i < objects_count; i++)
        {
            Obj o = objects[i];
            aabb_min_x = std::min(aabb_min_x, o.x - o.r - racer_radius);
            aabb_min_y = std::min(aabb_min_y, o.y - o.r - racer_radius);
            aabb_max_x = std::max(aabb_max_x, o.x + o.r + racer_radius);
            aabb_max_y = std::max(aabb_max_y, o.y + o.r + racer_radius);
        }

        map_start_x = aabb_min_x;
        map_start_y = aabb_min_y;
        i32 size_x = aabb_max_x - aabb_min_x + 1;
        i32 size_y = aabb_max_y - aabb_min_y + 1;

        map_cell_size = (std::max(size_x, size_y) + CellAxisCountTarget - 1) / CellAxisCountTarget;
        map_cells_x = (size_x + map_cell_size - 1) / map_cell_size;
        map_cells_y = (size_y + map_cell_size - 1) / map_cell_size;

        if(map_cells_x <= 0)
            map_cells_x = 1;
        if(map_cells_y <= 0)
            map_cells_y = 1;

        map = std::vector<std::vector<Obj>>(map_cells_x * map_cells_y, std::vector<Obj>());

		// Note: the order of objects in grid cells is important.
		// Objects that come first will be collided with preferentially.
        for(i32 i = 0; i < objects_count; i++)
        {
            Obj o = objects[i];

            i32 min_x = o.x - o.r - racer_radius;
            i32 min_y = o.y - o.r - racer_radius;
            i32 max_x = o.x + o.r + racer_radius;
            i32 max_y = o.y + o.r + racer_radius;

            i32 cell_min_x;
            i32 cell_min_y;
            i32 cell_max_x;
            i32 cell_max_y;

            to_cell_coords(min_x, min_y, cell_min_x, cell_min_y);
            to_cell_coords(max_x, max_y, cell_max_x, cell_max_y);

            for(i32 cell_y = cell_min_y; cell_y <= cell_max_y; cell_y++)
            {
                for(i32 cell_x = cell_min_x; cell_x <= cell_max_x; cell_x++)
                {
                    map.at(cell_x + cell_y * map_cells_x).push_back(o);
                }
            }
        }
    }

    const std::vector<Obj>& get_candidates(const i32 x, const i32 y) const
    {
        if(x < map_start_x || y < map_start_y || x >= map_start_x + map_cells_x * map_cell_size || y >= map_start_y + map_cells_y * map_cell_size)
        {
            // It is guaranteed that no object is outside the grid bounds.
            return std::vector<Obj>();
        }

        i32 cell_x;
        i32 cell_y;
        to_cell_coords(x, y, cell_x, cell_y);
        return map.at(cell_x + cell_y * map_cells_x);
    }
};

struct World
{
    i32 aabb_min_x;
    i32 aabb_min_y;
    i32 aabb_max_x;
    i32 aabb_max_y;
    std::vector<Obj> goals;
    Obj racer;

    GridLookup asteroid_grid;

    World(i32 aabb_min_x, i32 aabb_min_y, i32 aabb_max_x, i32 aabb_max_y, Obj* asteroids, i32 asteroids_count, Obj racer, Obj* goals, i32 goals_count)
        : asteroid_grid(asteroids, asteroids_count, racer.r), aabb_min_x(aabb_min_x), aabb_min_y(aabb_min_y), aabb_max_x(aabb_max_x), aabb_max_y(aabb_max_y), racer(racer)
    {
        for(i32 i = 0; i < goals_count; i++)
        {
            this->goals.push_back(goals[i]);
        }
    }
};

struct Vec
{
	i32 x = 0;
	i32 y = 0;

	Vec()
		: x(0), y(0)
	{
	}

	Vec(i32 x, i32 y)
		:x(x), y(y)
	{
	}
};

// From https://en.wikipedia.org/wiki/Integer_square_root
i64 isqrt(i64 s)
{
	// Zero yields zero
	// One yields one
	if (s <= 1) 
		return s;

	// Initial estimate (must be too high)
	i64 x0 = s / 2;

	// Update
	i64 x1 = (x0 + s / x0) / 2;

	while (x1 < x0)	// Bound check
	{
		x0 = x1;
		x1 = (x0 + s / x0) / 2;
	}
	return x0;
}

i64 euclidean_distance(i64 x1, i64 y1, i64 x2 = 0, i64 y2 = 0)
{
	i64 x = x1 - x2;
	i64 y = y1 - y2;
	return isqrt(x * x + y * y);
}

i64 distance_squared(i64 x1, i64 y1, i64 x2 = 0, i64 y2 = 0)
{
	i64 x = x1 - x2;
	i64 y = y1 - y2;
	return x * x + y * y;
}

i64 square(i64 x)
{
	return x * x;
}

struct Event // TODO: this is unused
{
	static constexpr i32 EVENT_TYPE_MOVE = 1;
	static constexpr i32 EVENT_TYPE_GOAL = 2;
	static constexpr i32 EVENT_TYPE_COLLISION_RESULT = 3;
	static constexpr i32 EVENT_TYPE_INVALID_INSTRUCTION = 4;

	i32 type;
	i32 data_x;
	i32 data_y;
	i32 data_vx;
	i32 data_vy;
	i32 data_index;
	i32 tick;
};

class Simulation
{
	Vec racer_pos;
	Vec racer_vel;
	World world;
	i32 tick_num;
	std::vector<bool> goal_states;
	i32 goals_reached_count;

	Simulation(World w)
		: racer_pos(), racer_vel(), world(w), tick_num(0), goal_states(w.goals.size(), false), goals_reached_count(0)
	{
	}

private:
	static const i32 INSTRUCTION_MIN = -128;
	static const i32 INSTRUCTION_MAX = 127;

	static const i32 DRAG_FRACTION_NOM = 9;
	static const i32 DRAG_FRACTION_DENOM = 10;

	static const i32 COLLISION_FRACTION_NOM = 1;
	static const i32 COLLISION_FRACTION_DENOM = 2;

	static const i32 MAX_COLLISION_RESOLUTIONS = 5;

	void reset()
	{
		racer_pos.x = world.racer.x;
		racer_pos.y = world.racer.y;
		racer_vel.x = 0;
		racer_vel.y = 0;
		tick_num = 0;
		
		for(i32 i = 0; i < goal_states.size(); i++)
		{
			goal_states[i] = false;
		}

		goals_reached_count = 0;
	}

	bool is_instruction_valid(const Vec& i) const
	{
		if(i.x > INSTRUCTION_MAX || i.y > INSTRUCTION_MAX || i.x < INSTRUCTION_MIN || i.y < INSTRUCTION_MIN)
			return false;
		
		// Note: 32 bit integer overflow cannot happen in this condition because the preceding condition guarantees that the values are clamped to -128..127
		if(i.x * i.x + i.y * i.y > INSTRUCTION_MAX * INSTRUCTION_MAX)
			return false;
		
		return true;
	}

	Event make_event_current_state() const
	{
		Event e;
		e.tick = tick_num;
		e.data_x = racer_pos.x;
		e.data_y = racer_pos.y;
		e.data_vx = racer_vel.y;
		e.data_vy = racer_vel.y;
		e.data_index = -1;
		e.type = Event::EVENT_TYPE_MOVE;
		return e;
	}

	/**
	 * Return true if a collision occured.
	*/
	bool try_push_out(Obj& o)
	{
		if(distance_squared(o.x - racer_pos.x, o.y - racer_pos.y) > square(world.racer.r + o.r))
		{
			return false;
		}

		i64 nx = racer_pos.x - o.x;
		i64 ny = racer_pos.y - o.y;

		i64 distance = euclidean_distance(nx, ny);
		i64 push_by = distance - (world.racer.r + o.r);

		racer_pos.x -= (nx * push_by / distance);
		racer_pos.y -= (ny * push_by / distance);

		return true;
	}

	void on_aabb_collision(bool& collided_this_iteration, bool& collided, std::vector<Event>& events) const
	{
		collided_this_iteration = true;
		collided = true;

		Event e = make_event_current_state();
		e.type = Event::EVENT_TYPE_COLLISION_RESULT;
		events.push_back(e);
	}

	/**
	 * Return true if any collision occured.
	*/
	bool resolve_collisions(std::vector<Event>& events)
	{
		bool collided = false;
		for(i32 colnum = 0; colnum < MAX_COLLISION_RESOLUTIONS; colnum++)
		{
			bool collided_this_iteration = false;

			auto candidates = world.asteroid_grid.get_candidates(racer_pos.x, racer_pos.y);

			for(i32 i = 0; i < candidates.size(); i++)
			{
				auto candidate = candidates.at(i);
				if(try_push_out(candidate))
				{
					collided_this_iteration = true;
					collided = true;

					Event e = make_event_current_state();
					e.type = Event::EVENT_TYPE_COLLISION_RESULT;
					events.push_back(e);

					break; // Note: we break the loop after first successful collision
				}
			}

			if (racer_pos.x - world.racer.r < world.aabb_min_x)
			{
				racer_pos.x = world.aabb_min_x + world.racer.r;
				on_aabb_collision(collided_this_iteration, collided, events);
			}
			if (racer_pos.y - world.racer.r < world.aabb_min_y)
			{
				racer_pos.y = world.aabb_min_y + world.racer.r;
				on_aabb_collision(collided_this_iteration, collided, events);
			}
			if (racer_pos.x + world.racer.r > world.aabb_max_x)
			{
				racer_pos.x = world.aabb_max_x - world.racer.r;
				on_aabb_collision(collided_this_iteration, collided, events);
			}
			if (racer_pos.y + world.racer.r > world.aabb_max_y)
			{
				racer_pos.y = world.aabb_max_y - world.racer.r;
				on_aabb_collision(collided_this_iteration, collided, events);
			}

			if(!collided_this_iteration)
			{
				break;
			}
		}
			
		if (collided)
		{
			racer_vel.x = racer_vel.x * COLLISION_FRACTION_NOM / COLLISION_FRACTION_DENOM;
			racer_vel.y = racer_vel.y * COLLISION_FRACTION_NOM / COLLISION_FRACTION_DENOM;

			// Patch speed to be correct in the last collision event
			events.at(events.size() - 1).data_vx = racer_vel.x;
			events.at(events.size() - 1).data_vy = racer_vel.y;
		}
		
		return collided;
	}

	/**
	 * Assumes a valid instruction.
	*/
	void move_racer(const Vec instruction)
	{
		racer_vel.x = racer_vel.x * DRAG_FRACTION_NOM / DRAG_FRACTION_DENOM;
		racer_vel.y = racer_vel.y * DRAG_FRACTION_NOM / DRAG_FRACTION_DENOM;

		racer_vel.x += instruction.x;
		racer_vel.y += instruction.y;

		racer_pos.x += racer_vel.x;
		racer_pos.y += racer_vel.y;
	}

	/**
	 * Returns true if a new goal is reached.
	*/
	bool check_goals(std::vector<Event>& events)
	{
		bool new_goal_reached = false;

		for(i32 i = 0; i < world.goals.size(); i++)
		{
			auto goal = world.goals.at(i);

			if(distance_squared(racer_pos.x, racer_pos.y, goal.x, goal.y) <= square(world.racer.r + goal.r))
			{
				if (!goal_states.at(i))
				{
					goals_reached_count++;
					new_goal_reached = true;
					goal_states.at(i) = true;
					Event e = make_event_current_state();
					e.type = Event::EVENT_TYPE_GOAL;
					e.data_index = i; // store goal index in the event
					events.push_back(e);
				}
			}
		}

        return new_goal_reached;
	}

	/**
	 * Returns true if the simulation should continue.
	*/
	bool tick(const Vec instruction, std::vector<Event>& events)
	{
		if(!is_instruction_valid(instruction))
		{
			Event e; // this event has the invalid instruction in its data x,y
			e.tick = tick_num;
			e.data_x = instruction.x;
			e.data_y = instruction.y;
			e.data_vx = 0;
			e.data_vy = 0;
			e.data_index = tick_num;
			e.type = Event::EVENT_TYPE_INVALID_INSTRUCTION;
			events.push_back(e);
			return false;
		}

		move_racer(instruction);

		tick_num++;
		events.push_back(make_event_current_state());

		bool collided = resolve_collisions(events);

		bool reached_goal = check_goals(events);
		
		return goals_reached_count == world.goals.size();
    }

public:
    /**
     * Simulates the given instructions. Instructions is an array of x,y values. Count is the number of instructions, not the number of ints in the array.
    */
    std::vector<Event> simulate(i32* instructions, i32 instruction_count)
    {
		std::vector<Event> events;
		reset();

		for(i32 i = 0; i < instruction_count; i++)
		{
			bool result = tick(Vec(instructions[i * 2 + 0], instructions[i * 2 + 1]), events);

			if(!result)
			{
				break;
			}
		}

		return events;
    }
};

void simulate()
{
    // TODO: a single function callable from javascript
}
