#![allow(dead_code)]
// ^ This is needed to avoid many warnings because this is not within a separate lib crate.

//! Provides tools for building KSP opendata generators, judges and solvers.
//!
//! # Usage
//! This module provides [`OpenData`], a simple way to build a single binary that includes
//! a generator, a judge, and solvers (or any subset of these), which makes it easy to reuse code.
//!
//! You can also use the individual building blocks instead:
//! * for *generators*, we have [`parse_seed`] to read seeds easily,
//! * for *solvers*, we provide nothing; they work with stdin/stdout directly,
//! * for *judges*, we have the [`judge`] module which handles everything for you,
//! * for all programs, we have [`dataset_dir`] used to get the directory with pre-built datasets.
//!
//! # Sharing code between gen, judge, and solve
//! This module provides a simple way to build a single binary
//! for any subset of these, which makes it easy to share code
//! between generators, judges, and solvers.
//!
//! ## Opendata config
//! Assuming the task crate is named `ksp`, the required configuration in
//! the opendata `config` file (version 2) looks like this:
//! ```
//! [prog_gen]
//! execute=ksp
//! arguments=--gen
//!
//! [prog_judge]
//! execute=ksp
//! arguments=--judge
//!
//! # You may define any amount of solvers in one project, add a configuration block for each
//! # defined solver, making sure the arguments match the defined name.
//! [prog_solve]
//! execute=ksp
//! arguments=--solve
//!
//! ```
//! This defines a `gen` program, a `solve` program and a `judge` program that may be used
//! in other parts of the config as usual.
//!
//! ## Rust code
//! After setting up the config, the Rust code is as simple as this:
//! ```rust
//! use rand::prelude::*;
//! use opendata::OpenData;
//! use opendata::judge::Verdict;
//!
//! fn main() {
//!     // Note that you may omit adding any of these parts.
//!     OpenData::new()
//!         .add_generator(gen)
//!         .add_judge(judge)
//!         .add_solver("--solve", solve)
//!         .add_solver("--solve2", solve2)
//!         .handle();
//! }
//!
//! fn gen(test: usize, seed: u64) {
//!     let mut rng = StdRng::seed_from_u64(seed);
//!     println!("this is an input");
//! }
//!
//! fn judge(
//!     test_name: &str,                      // Example inputs may have arbitrary names
//!     seed: Option<u64>,                    // Seed may be None for example inputs
//!     input_file: Option<File>,             // Only available if enabled in task config
//!     reference_output_file: Option<File>,  // Only available if enabled in task config
//! ) -> Verdict {
//!     // Submitted output is read from stdin.
//!     Verdict::wrong().message("The submitted path is too short.")
//! }
//!
//! fn solve() {
//!     // Input may be read from stdin.
//!     println!("this is a solution");
//! }
//!
//! fn solve2() {
//!     println!("this is a different solution");
//! }
//! ```
//!

use crate::opendata::judge::{input_filename, reference_output_filename, Verdict};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::num::ParseIntError;
use std::process::exit;

/// Parses a hexadecimal seed for use in generation.
pub fn parse_seed(hexadecimal: &str) -> Result<u64, ParseIntError> {
    u64::from_str_radix(hexadecimal, 16)
}

/// Returns the name of the directory with pre-built datasets.
/// `None` is returned if this is directory is not set; this is common when
/// running the program locally.
pub fn dataset_dir() -> Option<String> {
    env::var("DATASET_DIR").ok()
}

/// The building blocks of a judge.
///
/// # About judges
/// Judges are programs that decide whether a task solution is accepted or not.
/// This module is an abstraction over the KSP opendata interface for judges.
///
/// # Examples
/// The [`judge::Verdict`] is a builder for defining and delivering the output of a judge.
/// ```rust
/// use opendata::judge::Verdict;
///
/// fn judge(
///     test_name: &str,
///     seed: Option<u64>,
///     input_file: Option<File>,
///     reference_output_file: Option<File>,
/// ) -> Verdict {
///     Verdict::wrong().message("The submitted path is too short.")
/// }
/// ```
pub mod judge {
    use std::env;
    use std::process::exit;

    /// The type of a verdict.
    #[doc(hidden)]
    enum VerdictType {
        Correct,
        Wrong,
        InternalError,
    }

    /// A builder for creating judge results.
    ///
    /// # How to use
    /// This is a builder that allows specifying optional extras:
    /// - custom message, shown in the submit interface,
    /// - custom amount of points.
    ///
    /// This verdict may be used to quit the program by calling [`Verdict::deliver`],
    /// but if you are using [`super::OpenData`], you should instead just return the verdict.
    ///
    /// ## Possible verdicts
    /// |Verdict       |Instantiation|
    /// |--------------|----|
    /// |OK            |`Verdict::correct()`|
    /// |WRONG         |`Verdict::wrong()`|
    /// |Internal Error|`Verdict::internal_error()`|
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use opendata::judge::Verdict;
    ///
    /// // The solution is correct.
    /// let verdict = Verdict::correct();
    ///
    /// // The solution is correct and we want to change awarded
    /// // points.
    /// let verdict = Verdict::correct()
    ///     .override_points(14.0);
    ///
    /// // The solution is wrong.
    /// let verdict = Verdict::wrong()
    ///     .message("Your solution is not optimal.");
    ///
    /// // Any internal error in the judge.
    /// let verdict = Verdict::internal_error()
    ///     .message("Your solution is better than ours!");
    /// ```
    pub struct Verdict {
        verdict: VerdictType,
        message: Option<String>,
        points_override: Option<f64>,
    }

    impl Verdict {
        #[doc(hidden)]
        fn new(verdict: VerdictType) -> Self {
            Verdict {
                verdict,
                message: None,
                points_override: None,
            }
        }

        /// Creates a new builder for a "correct" verdict.
        #[must_use]
        pub fn correct() -> Self {
            Self::new(VerdictType::Correct)
        }

        /// Creates a new builder for a "wrong" verdict.
        #[must_use]
        pub fn wrong() -> Self {
            Self::new(VerdictType::Wrong)
        }

        /// Creates a new builder for an "internal error" verdict.
        #[must_use]
        pub fn internal_error() -> Self {
            Self::new(VerdictType::InternalError)
        }

        /// Sets the message that appears in the submit interface.
        /// The message should be a maximum of 255 bytes.
        #[must_use]
        pub fn message(mut self, message: &str) -> Self {
            self.message = Some(message.to_string());
            self
        }

        /// Sets an override for points, the contestant will be awarded this
        /// amount of points regardless of the config.
        ///
        /// Only used for correct verdicts.
        #[must_use]
        pub fn override_points(mut self, points: f64) -> Self {
            self.points_override = Some(points);
            self
        }

        /// Delivers the verdict, **ending the program**.
        ///
        /// Do not use directly if you are using [`super::OpenData`], return the verdict instead.
        pub fn deliver(self) {
            let exit_code = match self.verdict {
                VerdictType::Correct => 42,
                VerdictType::Wrong => 43,
                VerdictType::InternalError => 1,
            };

            let mut newline_printed = false;
            if let Some(message) = self.message {
                eprintln!("{}", message);
                newline_printed = true;
            }

            if let Some(points) = self.points_override {
                if !newline_printed {
                    eprintln!();
                }
                eprintln!("POINTS={:.1}", points);
            }

            exit(exit_code);
        }
    }

    /// Returns the filename of the input file - this is the file that contestants get.
    /// Requires `judge_needs_in=1` in the task config, otherwise `None` is returned.
    pub fn input_filename() -> Option<String> {
        env::var("TEST_INPUT").ok()
    }

    /// Returns the filename of the reference output file - this is the output calculated by us,
    /// **NOT** the contestant.
    /// Requires `judge_needs_out=1` in the task config, otherwise `None` is returned.
    pub fn reference_output_filename() -> Option<String> {
        env::var("TEST_OUTPUT").ok()
    }
}

type GeneratorHandler = fn(usize, u64);
type JudgeHandler = fn(&str, Option<u64>, Option<File>, Option<File>) -> Verdict;
type SolverHandler = fn();

/// Builder for a handler that calls the correct subprogram.
///
/// This is used in the `main()` function to run the correct program.
///
/// # Examples
/// For more detailed examples, see documentation of [`OpenData::add_generator`], [`OpenData::add_judge`] and [`OpenData::add_solver`].
///
/// ## Generator + Judge + Solver
/// ```rust
/// use rand::prelude::*;
/// use opendata::OpenData;
/// use opendata::judge::Verdict;
///
/// fn main() {
///     // Note that you may omit adding any of these parts.
///     OpenData::new()
///         .add_generator(gen)
///         .add_judge(judge)
///         .add_solver("--solve", solve)
///         .add_solver("--solve2", solve2)
///         .handle();
/// }
///
/// fn gen(test: usize, seed: u64) {
///     let mut rng = StdRng::seed_from_u64(seed);
///     println!("this is an input");
/// }
///
/// fn judge(
///     test_name: &str,                      // Example inputs may have arbitrary names
///     seed: Option<u64>,                    // Seed may be None for example inputs
///     input_file: Option<File>,             // Only available if enabled in task config
///     reference_output_file: Option<File>,  // Only available if enabled in task config
/// ) -> Verdict {
///     // Submitted output is read from stdin.
///     Verdict::wrong().message("The submitted path is too short.")
/// }
///
/// fn solve() {
///     // Input may be read from stdin.
///     println!("this is a solution");
/// }
///
/// fn solve2() {
///     println!("this is a different solution");
/// }
/// ```
pub struct OpenData {
    generate_handler: Option<GeneratorHandler>,
    judge_handler: Option<JudgeHandler>,
    solve_handlers: HashMap<String, SolverHandler>,
}

impl OpenData {
    /// Initializes a new OpenData builder.
    pub fn new() -> Self {
        OpenData {
            generate_handler: None,
            judge_handler: None,
            solve_handlers: HashMap::new(),
        }
    }

    /// Allows this program to act as an input generator.
    ///
    /// # About generators
    /// Generators generate task inputs, using the provided seed number to seed random number
    /// generation. It is **absolutely necessary** that the program returns
    /// the same input for the same test + seed combination.
    ///
    /// Note that you do not need to mix the test number with the seed in any way. While testing
    /// locally, you may notice the seed is always the same regardless of the test, often causing
    /// smaller tests to be substrings of bigger tests. This won't happen when ran
    /// by contestants, as a separate seed is used for each test case.
    ///
    /// ## Generator function arguments
    /// - `test:` [`usize`] &ndash; number of the subtask, **1-indexed**,
    /// - `seed:` [`u64`] &ndash; value that **must** be used to seed any random number generation.
    ///
    /// # Example generator
    ///
    /// ```rust
    /// use opendata::OpenData;
    ///
    /// fn main() {
    ///     OpenData::new()
    ///         .add_generator(gen)
    ///         .handle();
    /// }
    ///
    /// fn gen(test: usize, seed: u64) {
    ///     let mut rng = StdRng::seed_from_u64(seed);
    ///     let number: u32 = rng.gen();
    ///     println!("{}", number);
    /// }
    /// ```
    #[must_use]
    pub fn add_generator(mut self, f: GeneratorHandler) -> Self {
        assert!(self.generate_handler.is_none());
        self.generate_handler = Some(f);
        self
    }

    /// Allows this program to act as a judge.
    ///
    /// # About judges
    /// Judges are programs that decide whether a task solution is accepted or not.
    ///
    /// Note that most problems do not require a judge, some may be compared with a simple diff,
    /// we also have a token-based judge that allows detecting shuffled outputs. If you are not
    /// sure a judge is needed, ask before spending many hours implementing one.
    ///
    /// Judges are easy to get wrong. They must handle all possible edge cases, and there are
    /// usually many of those.
    ///
    /// # Judge function arguments
    /// * `test_name:` [`&str`] &ndash; name of the subtask, usually an unsigned that may be parsed, but for example inputs, this will be the name of the input.
    /// * `seed:` [`Option<u64>`] &ndash; the seed used to generate the task. May be `None` in case this is an example input, as they do not have seeds.
    /// * `input_file:` [`Option<File>`] &ndash; the input file; only available if enabled in task config (`judge_needs_in=1`), otherwise `None`.
    /// * `reference_output_file`: [`Option<File>`] &ndash; the reference output file generated by our solver; only available if enabled in task config (`judge_needs_out=1`), otherwise `None`.
    ///
    /// # Judged output
    /// The judged output is read from stdin.
    ///
    /// # Judge return value
    /// The judge function returns a [`judge::Verdict`]. Make sure to read the documentation for that type to
    /// see all available options.
    ///
    /// Note that you should **not** call [`Verdict::deliver`] in this handler; just return a [`Verdict`] instead.
    ///
    /// # Example judge
    ///
    /// ```rust
    /// use opendata::OpenData;
    ///
    /// fn main() {
    ///     OpenData::new()
    ///         .add_judge(judge)
    ///         .handle();
    /// }
    ///
    /// fn judge(
    ///     test_name: &str,                      // Examples may have arbitrary names
    ///     seed: Option<u64>,                    // Seed may be None for example inputs
    ///     input_file: Option<File>,             // Only available if enabled in task config
    ///     reference_output_file: Option<File>,  // Only available if enabled in task config
    /// ) -> Verdict {
    ///     Verdict::wrong().message("The submitted path is too short.")
    /// }
    /// ```
    #[must_use]
    pub fn add_judge(mut self, f: JudgeHandler) -> Self {
        assert!(self.judge_handler.is_none());
        self.judge_handler = Some(f);
        self
    }

    /// Allows this program to act as a solver.
    ///
    /// # About solvers
    /// Read the stdin, solve the task, write to stdout. It is as simple as that.
    ///
    /// # Multiple solvers
    /// Multiple solvers with different names may be defined, for example `--solve`, `--solve-slow`.
    /// The program may then be used with this name as an argument to run the corresponding solver.
    ///
    /// The task config for this example would look like this:
    /// ```
    /// [task]
    /// solutions=solve solve-slow
    ///
    /// [prog_solve]
    /// execute=ksp
    /// arguments=--solve
    ///
    /// [prog_solve-slow]
    /// execute=ksp
    /// arguments=--solve-slow
    /// ```
    ///
    /// Please use `--solve` as the name of the main solver.
    ///
    /// # Solver function arguments
    /// The solver has no arguments, the input is read from stdin.
    ///
    /// # Example solvers
    /// ```rust
    /// use opendata::OpenData;
    ///
    /// fn main() {
    ///     OpenData::new()
    ///         .add_solver("--solve", solve)
    ///         .add_solver("--solve2", solve2)
    ///         .handle();
    /// }
    ///
    /// fn solve() {
    ///     let mut bytes: Vec<u8> = Vec::new();
    ///     std::io::stdin().read_to_end(&mut bytes);
    ///     for byte in bytes.iter().rev() {
    ///         print!("{}", byte.to_ascii_lowercase())
    ///     }
    /// }
    ///
    /// fn solve2() {
    ///     let mut string = String::new();
    ///     std::io::stdin().read_to_string(&mut string);
    ///     println!("{}", string.chars().rev().collect::<String>());
    /// }
    /// ```
    #[must_use]
    pub fn add_solver(mut self, name: &str, f: SolverHandler) -> Self {
        assert_ne!(name, "--judge");
        assert_ne!(name, "--gen");
        assert!(!self.solve_handlers.contains_key(name));
        self.solve_handlers.insert(name.to_string(), f);
        self
    }

    #[doc(hidden)]
    fn print_usage(&self, args: &[String]) {
        let empty = self.generate_handler.is_none()
            && self.judge_handler.is_none()
            && self.solve_handlers.is_empty();
        if empty {
            println!("Invalid arguments.");
            println!("This program does nothing because no handlers were added; make sure to read the documentation.");
        } else {
            println!("Invalid arguments.");
            println!("This is a single binary for multiple programs, see usage:");
            if self.generate_handler.is_some() {
                println!("\tGenerator: {} --gen <test_id> <seed>", args[0]);
            }
            if self.judge_handler.is_some() {
                println!("\tJudge:     {} --judge <test_id> <seed>", args[0]);
            }
            for solve_handler in &self.solve_handlers {
                println!("\tSolver:    {} {}", args[0], solve_handler.0);
            }
        }
    }

    #[doc(hidden)]
    fn print_usage_and_exit(&self, args: &[String]) {
        self.print_usage(args);
        exit(1);
    }

    /// Choose a subprogram depending on arguments and run it. Exits the program afterwards.
    ///
    /// This finishes the [`OpenData`] configuration,
    /// Generally, this should be the last thing ran in the `main` function.
    ///
    /// # Example
    /// ```rust
    /// fn main() {
    ///     OpenData::new()
    ///         .add_generator(gen)
    ///         .add_judge(judge)
    ///         .add_solver("--solve", solve)
    ///         .handle();
    /// }
    /// ```
    pub fn handle(self) {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 2 {
            self.print_usage_and_exit(&args);
        }
        match args[1].as_str() {
            "--gen" => {
                if let Some(ref gen) = self.generate_handler {
                    if args.len() < 4 {
                        self.print_usage_and_exit(&args);
                    }
                    let (test_number, seed): (usize, u64) = (
                        args[2].parse().expect("Test number has to be an integer"),
                        parse_seed(&args[3]).expect("The seed format is incorrect"),
                    );

                    gen(test_number, seed);
                    exit(0);
                }
            }
            "--judge" => {
                if let Some(ref judge) = self.judge_handler {
                    if args.len() < 4 {
                        self.print_usage_and_exit(&args);
                    }
                    let test_name = &args[2];
                    let seed: Option<u64> = match args[3].as_str() {
                        "-" => None,
                        hexadecimal => {
                            Some(parse_seed(hexadecimal).expect("The seed format is incorrect"))
                        }
                    };

                    let input_name = input_filename();
                    let output_name = reference_output_filename();
                    let input_file =
                        input_name.map(|name| File::open(name).expect("Could not open input file"));
                    let reference_output_file = output_name
                        .map(|name| File::open(name).expect("Could not open output file"));

                    let verdict = judge(test_name, seed, input_file, reference_output_file);
                    verdict.deliver();
                } else {
                    self.print_usage_and_exit(&args);
                }
            }
            other => {
                if let Some(solve) = self.solve_handlers.get(other) {
                    solve();
                    exit(0);
                } else {
                    self.print_usage_and_exit(&args);
                }
            }
        }
    }
}
