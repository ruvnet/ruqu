use ruqu_independent_stabilizer::{Gate, Tableau};
use std::io::{self, Read};

const MAX_INPUT_BYTES: usize = 1024 * 1024;
const MAX_LINES: usize = 100_000;

enum Operation {
    Gate(Gate),
    Measure(usize),
    Reset(usize),
}

// SplitMix64: wrapping arithmetic, with one low output bit per M or R.
fn random_bit(state: &mut u64) -> bool {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((z ^ (z >> 31)) & 1) != 0
}

fn parse_program(input: &str, qubits: usize) -> Result<Vec<Operation>, String> {
    let mut program = Vec::new();
    for (line_index, line) in input.lines().enumerate() {
        let line_number = line_index + 1;
        if line_number > MAX_LINES {
            return Err(format!("input exceeds {MAX_LINES} lines"));
        }
        let source = line.split('#').next().unwrap_or("").trim();
        if source.is_empty() {
            continue;
        }
        let fields: Vec<&str> = source.split_whitespace().collect();
        let error = |message: &str| format!("line {line_number}: {message}");
        let operand_count = match fields[0] {
            "H" | "S" | "X" | "Y" | "Z" | "M" | "R" => 1,
            "CX" => 2,
            _ => return Err(error("unsupported operation")),
        };
        if fields.len() != operand_count + 1 {
            return Err(error("incorrect number of operands"));
        }
        let parse_qubit = |field: &str| -> Result<usize, String> {
            let q = field.parse::<usize>().map_err(|_| error("invalid qubit index"))?;
            if q >= qubits {
                return Err(error("qubit index out of range"));
            }
            Ok(q)
        };
        let q = parse_qubit(fields[1])?;
        let operation = match fields[0] {
            "H" => Operation::Gate(Gate::H(q)),
            "S" => Operation::Gate(Gate::S(q)),
            "X" => Operation::Gate(Gate::X(q)),
            "Y" => Operation::Gate(Gate::Y(q)),
            "Z" => Operation::Gate(Gate::Z(q)),
            "M" => Operation::Measure(q),
            "R" => Operation::Reset(q),
            "CX" => {
                let target = parse_qubit(fields[2])?;
                if q == target {
                    return Err(error("CX operands must be distinct"));
                }
                Operation::Gate(Gate::Cx(q, target))
            }
            _ => unreachable!(),
        };
        program.push(operation);
    }
    Ok(program)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: ruqu-independent-stabilizer <qubits> <seed>; read circuit from stdin".into());
    }
    let qubits = args[0].parse::<usize>().map_err(|_| "invalid qubit count")?;
    let mut seed = args[1].parse::<u64>().map_err(|_| "invalid unsigned 64-bit seed")?;
    let mut bytes = Vec::new();
    io::stdin().lock().take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes).map_err(|e| format!("stdin: {e}"))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!("input exceeds {MAX_INPUT_BYTES} bytes"));
    }
    let input = std::str::from_utf8(&bytes).map_err(|_| "input must be UTF-8")?;
    // Validate the entire program before allocating or executing the tableau.
    let program = parse_program(input, qubits)?;
    let mut tableau = Tableau::new(qubits).map_err(|e| format!("tableau: {e:?}"))?;
    let mut measurements = Vec::new();
    for operation in program {
        match operation {
            Operation::Gate(gate) => tableau.apply(gate).map_err(|e| format!("gate: {e:?}"))?,
            Operation::Measure(q) => {
                let value = tableau.measure_z(q, random_bit(&mut seed))
                    .map_err(|e| format!("measurement: {e:?}"))?;
                measurements.push((q, value));
            }
            Operation::Reset(q) => tableau.apply(Gate::Reset(q, random_bit(&mut seed)))
                .map_err(|e| format!("reset: {e:?}"))?,
        }
    }
    println!("backend=independent-rust-scalar");
    println!("qubits={qubits}");
    for (event, (qubit, value)) in measurements.iter().enumerate() {
        println!("measurement[{event}] qubit={qubit} value={}", u8::from(*value));
    }
    println!("measurements={}", measurements.len());
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
