//! cargo run -p ruqu-core --example hypergraph_decoder
use ruqu_core::hypergraph_decoder::{ErrorMechanism, HypergraphDecoder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // A single correlated fault flips three detectors and logical observable 0.
    let model = HypergraphDecoder::new(
        3,
        1,
        vec![
            ErrorMechanism {
                detectors: 0b111,
                logicals: 1,
                probability: 0.02,
            },
            ErrorMechanism {
                detectors: 0b001,
                logicals: 0,
                probability: 0.01,
            },
            ErrorMechanism {
                detectors: 0b110,
                logicals: 0,
                probability: 0.01,
            },
        ],
    )?;
    let decoded = model.decode(0b111, 1_000)?;
    assert_eq!(decoded.selected_mechanisms, 1);
    assert_eq!(decoded.logical_mask, 1);
    println!("{{\"model\":\"independent_Bernoulli_hypergraph\",\"selected_mechanisms\":{},\"logical_mask\":{},\"log_odds_cost\":{},\"visited_nodes\":{},\"logical_coset_marginalization\":false,\"hardware_run\":false}}",decoded.selected_mechanisms,decoded.logical_mask,decoded.log_odds_cost,decoded.visited_nodes);
    Ok(())
}
