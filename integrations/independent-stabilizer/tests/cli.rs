use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(qubits: &str, seed: &str, input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ruqu-independent-stabilizer"))
        .args([qubits, seed])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("launch CLI");
    // Oversized inputs can legitimately cause the CLI to close its input early.
    let write_result = child.stdin.take().unwrap().write_all(input);
    if let Err(error) = write_result {
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);
    }
    child.wait_with_output().expect("collect CLI output")
}

fn measurements(output: &Output) -> Vec<(usize, bool)> {
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let text = std::str::from_utf8(&output.stdout).unwrap();
    assert!(text.starts_with("backend=independent-rust-scalar\n"));
    text.lines()
        .filter(|line| line.starts_with("measurement["))
        .enumerate()
        .map(|(event, line)| {
            let fields: Vec<_> = line.split_whitespace().collect();
            assert_eq!(fields.len(), 3);
            assert_eq!(fields[0], format!("measurement[{event}]"));
            let qubit = fields[1].strip_prefix("qubit=").unwrap().parse().unwrap();
            let value = match fields[2] {
                "value=0" => false,
                "value=1" => true,
                value => panic!("invalid measurement value: {value}"),
            };
            (qubit, value)
        })
        .collect()
}

fn rejected(input: &[u8], expected_error: &str) {
    let output = run("2", "42", input);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty(), "failure must not emit partial results");
    let error = std::str::from_utf8(&output.stderr).unwrap();
    assert!(error.contains(expected_error), "{error}");
}

#[test]
fn bell_correlations_across_thirty_seeds() {
    let mut seen = [false; 2];
    for seed in 0..30 {
        let output = run("2", &seed.to_string(), b"H 0\nCX 0 1\nM 0\nM 1\n");
        let events = measurements(&output);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, 0);
        assert_eq!(events[1].0, 1);
        assert_eq!(events[0].1, events[1].1);
        seen[usize::from(events[0].1)] = true;
    }
    assert_eq!(seen, [true, true]);
}

#[test]
fn seeded_replay_is_identical() {
    let program = b"# replay\nH 0\nM 0\nR 0\nH 0 # second sample\nM 0\n\n";
    let first = run("1", "123456789", program);
    let second = run("1", "123456789", program);
    assert_eq!(measurements(&first).len(), 2);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    assert_eq!(first.status.code(), second.status.code());
}

#[test]
fn reset_returns_one_and_superposition_to_zero_without_extra_events() {
    for seed in 0..10 {
        let output = run("1", &seed.to_string(), b"X 0\nR 0\nM 0\nH 0\nR 0\nM 0\n");
        assert_eq!(measurements(&output), vec![(0, false), (0, false)]);
        assert!(std::str::from_utf8(&output.stdout).unwrap().ends_with("measurements=2\n"));
    }
}

#[test]
fn late_invalid_instruction_emits_no_partial_measurements() {
    rejected(b"H 0\nM 0\nT 1\n", "line 3: unsupported operation");
}

#[test]
fn unsupported_gate_and_bad_operands_are_rejected() {
    rejected(b"T 0\n", "unsupported operation");
    rejected(b"M 2\n", "qubit index out of range");
    rejected(b"CX 0 0\n", "CX operands must be distinct");
    rejected(b"H 0 1\n", "incorrect number of operands");
    rejected(b"M -1\n", "invalid qubit index");
}

#[test]
fn byte_and_line_limits_are_enforced() {
    rejected(&vec![b' '; 1024 * 1024 + 1], "input exceeds 1048576 bytes");
    rejected(&vec![b'\n'; 100_001], "input exceeds 100000 lines");
    // Exact limits are accepted when the program contains only whitespace.
    assert!(run("1", "0", &vec![b' '; 1024 * 1024]).status.success());
    assert!(run("1", "0", &vec![b'\n'; 100_000]).status.success());
}

#[test]
fn malformed_utf8_and_arguments_are_rejected() {
    rejected(&[0xff], "input must be UTF-8");
    for (qubits, seed) in [("invalid", "0"), ("2", "-1"), ("2", "18446744073709551616")] {
        let output = run(qubits, seed, b"");
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
}
