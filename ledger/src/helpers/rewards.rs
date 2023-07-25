// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkVM library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use console::{account::Address, network::prelude::*};
use ledger_block::Ratify;

/// A safety bound (sanity-check) for the proving reward.
const MAX_PROVING_REWARD: u128 = 3_000_000_000;

/// Returns the proving rewards for a given coinbase reward and list of prover solutions.
///
/// The prover reward is defined as:
///   1/2 * coinbase_reward * (proof_target / combined_proof_target)
///   = (coinbase_reward * proof_target) / (2 * combined_proof_target)
pub fn proving_rewards<N: Network>(
    proof_targets: Vec<(Address<N>, u128)>,
    coinbase_reward: u64,
    combined_proof_target: u128,
) -> Vec<Ratify<N>> {
    // (Debug Mode) Ensure the combined proof target is equal to the sum of the proof targets.
    debug_assert_eq!(combined_proof_target, proof_targets.iter().map(|(_, t)| t).sum::<u128>());

    // If the list of solutions is empty, return an empty vector.
    if proof_targets.is_empty() {
        return Vec::new();
    }

    // Initialize a vector to store the proving rewards.
    let mut rewards = Vec::with_capacity(proof_targets.len());

    // Calculate the rewards for the individual provers.
    for (address, proof_target) in proof_targets {
        // Compute the numerator.
        let numerator = (coinbase_reward as u128).saturating_mul(proof_target);
        // Compute the denominator.
        // Note: We guarantee this denominator cannot be 0 (to prevent a div by 0).
        let denominator = combined_proof_target.saturating_mul(2).max(1);
        // Compute the quotient.
        let quotient = numerator.saturating_div(denominator);
        // Ensure the proving reward is within a safe bound.
        if quotient > MAX_PROVING_REWARD {
            error!("Prover reward ({quotient}) is too large - skipping solution from {address}");
            continue;
        }
        // Cast the proving reward as a u64.
        // Note: This '.expect' is guaranteed to be safe, as we ensure the quotient is within a safe bound.
        let prover_reward = u64::try_from(quotient).expect("Prover reward is too large");
        // If there is a proving reward, append it to the vector.
        if prover_reward > 0 {
            // Append the proving reward to the vector.
            rewards.push(Ratify::ProvingReward(address, prover_reward));
        }
    }

    // Return the proving rewards.
    rewards
}

#[cfg(test)]
mod tests {
    use super::*;
    use console::{prelude::TestRng, types::Group};

    type CurrentNetwork = console::network::Testnet3;

    const ITERATIONS: usize = 1000;

    #[test]
    fn test_proving_rewards_cannot_exceed_coinbase_reward() {
        let rng = &mut TestRng::default();

        for _ in 0..ITERATIONS {
            // Sample a random address.
            let address = Address::new(Group::rand(rng));
            // Sample a random coinbase reward.
            let coinbase_reward = rng.gen_range(0..u64::try_from(MAX_PROVING_REWARD).unwrap());
            // Check that a maxed out proof target fails.
            let rewards =
                proving_rewards::<CurrentNetwork>(vec![(address, u64::MAX as u128)], coinbase_reward, u64::MAX as u128);
            assert_eq!(rewards.len(), 1);
            assert!(matches!(rewards[0], Ratify::ProvingReward(..)));
            if let Ratify::ProvingReward(candidate_address, candidate_amount) = rewards[0] {
                assert_eq!(candidate_address, address);
                assert!(candidate_amount <= coinbase_reward);
            }
        }
    }

    #[test]
    fn test_proving_rewards_is_empty() {
        let rng = &mut TestRng::default();
        // Sample a random address.
        let address = Address::new(Group::rand(rng));

        // Compute the proving rewards (empty).
        let rewards = proving_rewards::<CurrentNetwork>(vec![], rng.gen(), 0);
        assert!(rewards.is_empty());

        // Check that a maxed out coinbase reward, returns empty.
        let rewards = proving_rewards::<CurrentNetwork>(vec![(address, 2)], u64::MAX, 2);
        assert!(rewards.is_empty());

        // Ensure a 0 coinbase reward case is empty.
        let rewards = proving_rewards::<CurrentNetwork>(vec![(address, 2)], 0, 2);
        assert!(rewards.is_empty());

        // Ensure a proving reward that is too large, renders no rewards.
        for _ in 0..ITERATIONS {
            // Sample a random address.
            let address = Address::new(Group::rand(rng));
            // Sample a random overly-large coinbase reward.
            let coinbase_reward = rng.gen_range(u64::try_from(MAX_PROVING_REWARD).unwrap()..u64::MAX);
            // Sample a random proof target.
            let proof_target = rng.gen_range(0..u64::MAX as u128);
            // Check that a maxed out proof target fails.
            let rewards =
                proving_rewards::<CurrentNetwork>(vec![(address, proof_target)], coinbase_reward, proof_target);
            assert!(rewards.is_empty());
        }
    }
}
