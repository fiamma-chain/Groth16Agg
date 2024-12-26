use ark_bn254::{Bn254, Fr};
use ark_ff::One;
use ark_groth16::{prepare_verifying_key, Groth16};
use snarkpack;
use snarkpack::transcript::Transcript;

mod constraints;
use crate::constraints::Benchmark;
use rand_core::SeedableRng;

#[test]
fn groth16_aggregation() {
    let num_constraints = 1000;
    let nproofs = 8;
    let mut rng = rand_chacha::ChaChaRng::seed_from_u64(1u64);
    let params = {
        let c = Benchmark::<Fr>::new(num_constraints);
        Groth16::<Bn254>::generate_random_parameters_with_reduction(c, &mut rng).unwrap()
    };
    // prepare the verification key
    let pvk = prepare_verifying_key(&params.vk);
    // prepare the SRS needed for snarkpack - specialize after to the right
    // number of proofs
    let srs = snarkpack::srs::setup_fake_srs::<Bn254, _>(&mut rng, nproofs);
    let (prover_srs, ver_srs) = srs.specialize(nproofs);
    // create all the proofs
    let proofs = (0..nproofs)
        .map(|_| {
            let c = Benchmark::new(num_constraints);
            Groth16::<Bn254>::create_random_proof_with_reduction(c, &params, &mut rng)
                .expect("proof creation failed")
        })
        .collect::<Vec<_>>();
    // verify we can at least verify one
    let inputs: Vec<_> = [Fr::one(); 2].to_vec();
    let all_inputs = (0..nproofs).map(|_| inputs.clone()).collect::<Vec<_>>();
    let r = Groth16::<Bn254>::verify_proof(&pvk, &proofs[1], &inputs).unwrap();
    assert!(r);

    let mut prover_transcript = snarkpack::transcript::new_merlin_transcript(b"test aggregation");
    prover_transcript.append(b"public-inputs", &all_inputs);
    let aggregate_proof = snarkpack::aggregate_proofs(&prover_srs, &mut prover_transcript, &proofs)
        .expect("error in aggregation");

    let mut ver_transcript = snarkpack::transcript::new_merlin_transcript(b"test aggregation");
    ver_transcript.append(b"public-inputs", &all_inputs);
    snarkpack::verify_aggregate_proof(
        &ver_srs,
        &pvk,
        &all_inputs,
        &aggregate_proof,
        &mut rng,
        &mut ver_transcript,
    )
    .expect("error in verification");
}
