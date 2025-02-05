/// Naprosto shamelessly ukradeno/poupraveno z 34-2-4 (díky Jirko)
use crate::opendata::judge::Verdict;
use crate::simulation::*;
use std::fs::File;
use std::io::{stdin, BufRead};
use std::path::PathBuf;


enum Task {
    Sprint,
    Marathon,
}

pub fn judge(
    test_name: &str,
    _seed: Option<u64>,
    _input_file: Option<File>,
    _reference_output_file: Option<File>,
) -> Verdict {
    let mut simulation;
    let task;

    // first test is sprint
    if test_name == "sprint" {
        task = Task::Sprint;
        simulation = Simulation::load(&PathBuf::from("TODO"));
    } else if test_name == "marathon" {
        task = Task::Marathon;
        simulation = Simulation::load(&PathBuf::from("TODO"));
    } else {
        return Verdict::internal_error().message(&format!("Špatné jméno úlohy '{}'", test_name));
    }

    let instructions = match read_submitted_output(stdin().lock()) {
        Ok(output) => output,
        Err(OutputReadError::IoError(_)) => {
            return Verdict::internal_error().message("Chyba při čtení souboru.");
        }
        Err(OutputReadError::FirstLineError) => {
            return Verdict::wrong().message("První řádek neobsahuje počet instrukcí!");
        }
        Err(OutputReadError::NumberTypeError(line)) => {
            return Verdict::wrong()
                .message(&format!("Instrukce na řádku {} nemá správný typ!", line));
        }
        Err(OutputReadError::NumberCountError(line)) => {
            return Verdict::wrong().message(&format!(
                "Instrukce na řádku {} nemá správný počet čísel!",
                line
            ));
        }
        Err(OutputReadError::LengthError(line)) => {
            return Verdict::wrong().message(&format!(
                "Euklidovská vzdálenost instrukce na řádku {} je větší než povolená!",
                line
            ));
        }
        Err(OutputReadError::InstructionCountError) => {
            return Verdict::wrong().message("Nesedí počet instrukcí!");
        }
    };

    // brrrrrrr
    simulation.simulate(&instructions);

    fn format_unreached_goals(arr: &Vec<bool>) -> String {
        arr.iter()
            .enumerate()
            .filter_map(|(i, &b)| if !b { Some((i + 1).to_string()) } else { None })
            .collect::<Vec<_>>()
            .join(", ")
    }

    if !simulation.finished() {
        return Verdict::wrong().message(&format!(
            "Po provedení instrukcí nebyly dosaženy cíle {}!",
            format_unreached_goals(&simulation.reached_goals),
        ));
    }

    Verdict::correct()
        .override_points(points(instructions.len(), task))
        .message(&format!("Úspěšný let!", ))
}

fn points(length: usize, task: Task) -> f64 {
    const MAX_POINTS: f64 = 12.0;

    // tyhle hodnoty jsou hodné dobré baseline řešení obou úložek
    // pokud někdo dosáhne těch, tak max body, jinak exponenciálně klesá skóre
    let good_length = match task {
        Task::Sprint => 1151,
        Task::Marathon => 14207,
    };

    if length <= good_length {
        MAX_POINTS
    } else {
        good_length as f64 / (length as f64 - good_length as f64)
    }
}

enum ValidationError {
    VertexOutOfBounds(usize),
    VertexBlocked(usize, usize),
    EdgeNotFound(usize, usize),
}

enum OutputReadError {
    IoError(std::io::Error),
    FirstLineError,
    NumberTypeError(usize),  // čísla nejsou i8
    NumberCountError(usize), // nemáme x and y (máme víc/míň čísel)
    LengthError(usize),      // instrukce není normalizovaná (viz zadání)
    InstructionCountError,   // první řadek není počet instrukcí
}

impl From<std::io::Error> for OutputReadError {
    fn from(e: std::io::Error) -> Self {
        OutputReadError::IoError(e)
    }
}

fn read_submitted_output<TReader: BufRead>(
    reader: TReader,
) -> Result<Vec<Instruction>, OutputReadError> {
    let mut length = None;
    let mut instructions = Vec::new();

    for (i, line) in reader.lines().enumerate() {
        if i == 0 {
            length = match line?.parse::<usize>() {
                Ok(len) => Some(len),
                Err(_) => return Err(OutputReadError::FirstLineError),
            };
        } else {
            let line = line?;

            let parts = line.split_whitespace().collect::<Vec<&str>>();

            if parts.len() != 2 {
                return Err(OutputReadError::NumberCountError(i));
            }

            let mut parsed = vec![];
            for part in parts {
                parsed.push(match part.parse::<InstType>() {
                    Ok(edge) => edge,
                    Err(_) => return Err(OutputReadError::NumberTypeError(i)),
                });
            }

            let instruction = Instruction::new(parsed[0], parsed[1]);

            // pokud se instrukce přeškálovala, tak nebyla správné velikosti a nebereme
            if instruction.vx != parsed[0] || instruction.vy != parsed[1] {
                return Err(OutputReadError::LengthError(i));
            }

            instructions.push(instruction);
        }
    }

    let length = match length {
        Some(len) => len,
        None => return Err(OutputReadError::FirstLineError),
    };

    if instructions.len() != length {
        Err(OutputReadError::InstructionCountError)
    } else {
        Ok(instructions)
    }
}
