use crate::Die;

const STATE_COUNT: usize = 81;  // 3^4 = 81

// We track counts only for faces {2,3,4,6}.
// Each can be 0, 1, or 2 (3+ would already score -> not bust).
//
// State is encoded as base-3 number:
// (c2, c3, c4, c6) where each ∈ {0,1,2}
// Total states = 3^4 = 81.

fn encode(c2: u8, c3: u8, c4: u8, c6: u8) -> usize {
    (c2 as usize)
        + 3 * (c3 as usize)
        + 9 * (c4 as usize)
        + 27 * (c6 as usize)
}

fn decode(idx: usize) -> (u8, u8, u8, u8) {
    let c2 = (idx % 3) as u8;
    let c3 = ((idx / 3) % 3) as u8;
    let c4 = ((idx / 9) % 3) as u8;
    let c6 = ((idx / 27) % 3) as u8;
    (c2, c3, c4, c6)
}

/// Computes probability that a single roll of 6 dice is a bust.
///
/// Bust means:
/// - no 1 (or Devil Head if the die has is_devil = true)
/// - no 5
/// - no face appears 3 or more times
///
/// Note: Straights are irrelevant for bust because every straight includes 5 (and some include 1),
/// which already prevents bust under the above rules.
pub fn probability_bust(dice: &[Die], counts: &[u8]) -> f64 {
    assert_eq!(dice.len(), counts.len());

    let total: usize = counts.iter().map(|&c| c as usize).sum();
    assert_eq!(total, 6, "counts must sum to 6 dice");

    // dp[state] = probability of reaching this state after processing some dice
    let mut dp = [0.0f64; STATE_COUNT];
    dp[encode(0, 0, 0, 0)] = 1.0;

    for (die, &count) in dice.iter().zip(counts.iter()) {
        // Pre-read probabilities for faces we allow in bust states:
        // face indices: 1->2, 2->3, 3->4, 5->6
        let p2 = die.probs[1];
        let p3 = die.probs[2];
        let p4 = die.probs[3];
        let p6 = die.probs[5];

        for _ in 0..count {
            let mut next = [0.0f64; STATE_COUNT];

            for state in 0..STATE_COUNT {
                let prob = dp[state];
                if prob == 0.0 {
                    continue;
                }

                let (c2, c3, c4, c6_) = decode(state);

                // Only transitions that keep us in "bust-possible" space:
                // rolling 1/devil or 5 eliminates the branch (scoring),
                // rolling a 3rd copy of any face also eliminates it (triple scoring).

                if p2 != 0.0 && c2 < 2 {
                    next[encode(c2 + 1, c3, c4, c6_)] += prob * p2;
                }
                if p3 != 0.0 && c3 < 2 {
                    next[encode(c2, c3 + 1, c4, c6_)] += prob * p3;
                }
                if p4 != 0.0 && c4 < 2 {
                    next[encode(c2, c3, c4 + 1, c6_)] += prob * p4;
                }
                if p6 != 0.0 && c6_ < 2 {
                    next[encode(c2, c3, c4, c6_ + 1)] += prob * p6;
                }
            }

            dp = next;
        }
    }

    dp.iter().sum()
}