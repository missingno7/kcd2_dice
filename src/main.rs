use bust::probability_bust;
use new_probs::{calc_score, enumerate_histogram_probabilities, evaluate_histogram, print_histogram_ev, print_score_distribution};
use render::{color_for_faces, render_probability_image_216_from_counts};
use straights::{print_straight_breakdown, straight_terms_exclusive};

mod bust;
mod straights;
mod render;
mod new_probs;

#[derive(Clone)]
pub struct Die {
    probs: [f64; 6],
    name: String,
    pub is_devil: bool,
}

impl Die {
    pub fn new(weights: [u32; 6], name: String) -> Self {
        let sum: u32 = weights.iter().sum();

        assert!(sum > 0, "Die weights must not sum to zero");

        let sum_f = sum as f64;

        let probs = [
            weights[0] as f64 / sum_f,
            weights[1] as f64 / sum_f,
            weights[2] as f64 / sum_f,
            weights[3] as f64 / sum_f,
            weights[4] as f64 / sum_f,
            weights[5] as f64 / sum_f,
        ];

        Self { probs, name, is_devil: false }
    }

    pub fn devil() -> Self {
       let mut new_die = Self::new([1,1,1,1,1,1], "Devil".to_string());
       new_die.is_devil=true;
        return new_die
    }
}


fn probability_exact(
    dice: &[Die],
    counts: &[u8],
    face: usize,
    target: usize,
) -> f64 {
    let mut dp = [0.0f64; 7];
    dp[0] = 1.0;

    for (die_index, &count) in counts.iter().enumerate() {
        let mut p = dice[die_index].probs[face];
        if dice[die_index].is_devil && face != 0 {
            p+=dice[die_index].probs[0];
        }

        let q = 1.0 - p;

        for _ in 0..count {
            let mut next = [0.0f64; 7];

            for r in 0..=6 {
                if dp[r] == 0.0 {
                    continue;
                }

                // nepadne
                next[r] += dp[r] * q;

                // padne
                if r + 1 <= 6 {
                    next[r + 1] += dp[r] * p;
                }
            }

            dp = next;
        }
    }

    dp[target]
}

#[derive(Debug, Clone)]
pub struct EvTerm {
    pub face: usize,
    // 0..5 (1..6)
    pub count: usize,
    // 0..6
    pub p: f64,
    pub score: u32,
    pub ev: f64,
}


/// Expected score of the *first roll* (one throw of 6 dice),
/// ignoring straights and Devil's Head, using KCD2 scoring:
/// - single 1 = 100
/// - single 5 = 50
/// - three-of-a-kind: 1..6 => {1000,200,300,400,500,600}
/// - each additional die after 3 doubles the value (4x = 2×, 5x = 4×, 6x = 8×)
///
/// This assumes scoring is additive across faces (true here since we ignore straights/jokers).
/// Expected score of the *first roll* (one throw of 6 dice),
/// computed exactly via histogram enumeration (order-independent),
/// using the full `calc_score` logic (including straights etc.).
pub fn expected_first_roll_score(dice_types: &[Die], counts_by_type: &[u8]) -> f64 {
    debug_assert_eq!(
        counts_by_type.iter().map(|&c| c as usize).sum::<usize>(),
        6,
        "counts must sum to 6 dice"
    );

    let hist = enumerate_histogram_probabilities(dice_types, counts_by_type);

    let mut ev = 0.0;
    for (counts_u8, p) in hist {
        let counts_usize: [usize; 6] = [
            counts_u8[0] as usize,
            counts_u8[1] as usize,
            counts_u8[2] as usize,
            counts_u8[3] as usize,
            counts_u8[4] as usize,
            counts_u8[5] as usize,
        ];

        let score = calc_score(&counts_usize) as f64;
        ev += p * score;
    }

    ev
}

/// Score contributed by a single face (0..5 == 1..6) given that it appears `count` times in the roll.
/// Returns an integer score (points).
fn score_for_face_count(face: usize, count: usize) -> u32 {
    // face: 0..5 represents value 1..6
    let value = face + 1;

    // Singles
    if value == 1 && count < 3 {
        return (count as u32) * 100;
    }
    if value == 5 && count < 3 {
        return (count as u32) * 50;
    }

    // Three-of-a-kind base values
    let base3: u32 = match value {
        1 => 1000,
        2 => 200,
        3 => 300,
        4 => 400,
        5 => 500,
        6 => 600,
        _ => unreachable!(),
    };

    if count < 3 {
        return 0;
    }

    // After 3, each additional die doubles the value
    let multiplier: u32 = 1u32 << ((count - 3) as u32); // 3->1, 4->2, 5->4, 6->8
    base3 * multiplier
}

pub fn expected_first_roll_score_with_breakdown(
    dice: &[Die],
    counts: &[u8],
) -> (f64, Vec<EvTerm>) {
    debug_assert_eq!(
        counts.iter().map(|&c| c as usize).sum::<usize>(),
        6,
        "counts must sum to 6 dice"
    );

    let mut total_ev = 0.0;
    let mut terms: Vec<EvTerm> = Vec::new();

    for face in 0..6 {
        for count in 0..=6 {
            let score = score_for_face_count(face, count);
            if score == 0 {
                continue;
            }

            let p = probability_exact(dice, counts, face, count);
            if p == 0.0 {
                continue;
            }

            let ev = p * score as f64;
            total_ev += ev;

            terms.push(EvTerm {
                face,
                count,
                p,
                score,
                ev,
            });
        }
    }

    (total_ev, terms)
}

/// Convenience printer (top N)
pub fn print_ev_breakdown(terms: &[EvTerm], top_n: usize) {
    for (_i, t) in terms.iter().take(top_n).enumerate() {
        println!(
            "{}x{}  p={:.2}%  score={}  ev={:.3}",
            t.count,
            t.face + 1,
            t.p * 100.0,
            t.score,
            t.ev
        );
    }
}

pub fn format_dice_set(dice_types: &[Die], counts: &[u8]) -> String {
    assert_eq!(dice_types.len(), counts.len());

    let mut parts: Vec<String> = Vec::new();

    for (die, &count) in dice_types.iter().zip(counts.iter()) {
        if count > 0 {
            parts.push(format!("{}x {}", count, die.name));
        }
    }

    parts.join(", ")
}

pub fn evaluate_and_print(dice_types: &[Die], counts: &[u8]) {
    assert_eq!(
        dice_types.len(),
        counts.len(),
        "dice_types and counts must have same length"
    );

    // Base EV (singles + triples etc.)
    let (base_ev,mut breakdown) =
        expected_first_roll_score_with_breakdown(dice_types, counts);

    // Sort descending by EV contribution
    breakdown.sort_by(|a, b| b.ev.partial_cmp(&a.ev).unwrap_or(std::cmp::Ordering::Equal));


    // Bust probability
    let bust = probability_bust(dice_types, counts);

    // Straight EV (exclusive, no double counting)
    let straight_terms = straight_terms_exclusive(dice_types, counts);
    let straight_ev: f64 = straight_terms.iter().map(|t| t.ev).sum();

    // Total EV including straights
    let total_ev = base_ev + straight_ev;

    println!(
        "{} = {:.2} (total={:.2}, straights={:.2}), bust={:.2}%",
        format_dice_set(dice_types, counts),
        total_ev,
        base_ev,
        straight_ev,
        bust * 100.0
    );

    // Print base breakdown
    print_ev_breakdown(&breakdown, usize::MAX);

    // Print straight breakdown
    println!("--- Straights (exclusive) ---");
    print_straight_breakdown(&straight_terms);

    println!("---------------------------------------------");
}



/// Finds the best multiset of exactly 6 dice from the available types with per-type limits.
/// Returns (best_score, best_counts).
pub fn find_best_dice_set(
    available_dice: &[Die],
    available_counts: &[u8],
) -> (f64, Vec<u8>) {
    assert_eq!(
        available_dice.len(),
        available_counts.len(),
        "available_dice and available_counts must have same length"
    );

    let n = available_dice.len();
    let mut current = vec![0u8; n];
    let mut best_counts = vec![0u8; n];
    let mut best_score = f64::NEG_INFINITY;

    fn dfs(
        i: usize,
        remaining: u8,
        dice: &[Die],
        limits: &[u8],
        current: &mut [u8],
        best_score: &mut f64,
        best_counts: &mut [u8],
    ) {
        if i == dice.len() {
            if remaining == 0 {
                let score = expected_first_roll_score(dice, current);
                if score > *best_score {
                    *best_score = score;
                    best_counts.copy_from_slice(current);
                        println!(
        "Best = {:.2}  ({})",
        best_score,
        format_dice_set(&dice, &current)
    );
                }
            }
            return;
        }

        // Choose how many dice of type i to take.
        let max_take = limits[i].min(remaining);
        for take in 0..=max_take {
            current[i] = take;
            dfs(
                i + 1,
                remaining - take,
                dice,
                limits,
                current,
                best_score,
                best_counts,
            );
        }
        current[i] = 0;
    }

    dfs(
        0,
        6,
        available_dice,
        available_counts,
        &mut current,
        &mut best_score,
        &mut best_counts,
    );

    (best_score, best_counts)
}

/// Converts (available_dice, best_counts) into (dice_types, counts)
/// suitable for evaluate_and_print(&dice_types, &counts).
pub fn selection_from_counts(
    available_dice: &[Die],
    best_counts: &[u8],
) -> (Vec<Die>, Vec<u8>) {
    assert_eq!(
        available_dice.len(),
        best_counts.len(),
        "available_dice and best_counts must have same length"
    );

    let mut dice_types = Vec::new();
    let mut counts = Vec::new();

    for (die, &c) in available_dice.iter().zip(best_counts.iter()) {
        if c > 0 {
            dice_types.push(die.clone());
            counts.push(c);
        }
    }

    (dice_types, counts)
}



fn main() {

    //println!("{:?}", color_for_faces(&[0,1,2,3,3,3],255));



    //return;
let aranka_die = Die::new([6,1,6,1,6,1], "Aranka's die".to_string());
let cautious_cheaters_die = Die::new([5,3,2,3,5,3], "Cautious cheater's die".to_string());
let lu_ci_die = Die::new([3,3,3,3,3,6], "Lu/Ci die".to_string());
//let devils_head_die = Die::devil();
let die_of_misfortune = Die::new([1,5,5,5,5,1], "Die of misfortune".to_string());
let even_die = Die::new([2,8,2,8,2,8], "Even die".to_string());
let favourable_die = Die::new([6,0,1,1,6,4], "Favourable die".to_string());
let fer_die = Die::new([3,3,3,3,3,5], "Fer die".to_string());
let greasy_die = Die::new([3,2,3,2,3,4], "Greasy die".to_string());
let grimy_die = Die::new([1,5,1,1,7,1], "Grimy die".to_string());
let grozavs_lucky_die = Die::new([1,10,1,1,1,1], "Grozav's lucky die".to_string());
let heavenly_kingdom_die = Die::new([7,2,2,2,2,4], "Heavenly Kingdom die".to_string());
let holy_trinity_die = Die::new([4,5,7,1,1,1], "Holy Trinity die".to_string());
//let hugos_die = Die::new([1,1,1,1,1,1], "Hugo's Die".to_string());
let kings_die = Die::new([4,6,7,8,4,3], "King's die".to_string());
let lousy_gamblers_die = Die::new([2,3,2,3,7,3], "Lousy gambler's die".to_string());
//let lu_die = Die::new([3,3,3,3,3,6], "Lu die".to_string());
let lucky_die = Die::new([6,1,2,3,4,6], "Lucky Die".to_string());
let mathematicians_die = Die::new([4,5,6,7,1,1], "Mathematician's Die".to_string());
//let molar_die = Die::new([1,1,1,1,1,1], "Molar die".to_string());
let mother_of_pearl_die = Die::new([3,1,1,1,3,3], "Mother-of-pearl die".to_string());
let odd_die = Die::new([8,2,8,2,8,2], "Odd die".to_string());
let ordinary_die = Die::new([1,1,1,1,1,1], "Ordinary die".to_string());
let painted_die = Die::new([3,1,1,1,6,3], "Painted die".to_string());
let painters_die = Die::new([1,3,2,2,2,1], "Painter's die".to_string());
let pie_die = Die::new([6,1,3,3,0,0], "Pie die".to_string());
//let premolar_die = Die::new([1,1,1,1,1,1], "Premolar die".to_string());
let sad_greasers_die = Die::new([6,6,1,1,6,3], "Sad Greaser's Die".to_string());
let saint_antiochus_die = Die::new([3,1,6,1,1,3], "Saint Antiochus' die".to_string());
let shrinking_die = Die::new([2,1,1,1,1,3], "Shrinking die".to_string());
//let st_stephens_die = Die::new([1,1,1,1,1,1], "St. Stephen's die".to_string());
let strip_die = Die::new([4,2,2,2,3,3], "Strip die".to_string());
let three_die = Die::new([2,1,4,1,2,1], "Three die".to_string());
let unbalanced_die = Die::new([3,4,1,1,2,1], "Unbalanced Die".to_string());
let unlucky_die = Die::new([1,3,2,2,2,1], "Unlucky die".to_string());
let wagoners_die = Die::new([1,5,6,2,2,2], "Wagoner's Die".to_string());
let weighted_die = Die::new([10,1,1,1,1,1], "Weighted die".to_string());
//let wisdom_tooth_die = Die::new([1,1,1,1,1,1], "Wisdom tooth die".to_string());


//     let all_dice_with_counts = vec![
//     (aranka_die, 1),
//     (cautious_cheaters_die, 1),
//     //(ci_die, 1),
//     //(devils_head_die, 1),
//     //(die_of_misfortune, 1),
//     //(even_die, 1),
//     (favourable_die, 6),
//     //(fer_die, 1),
//     //(greasy_die, 1),
//     //(grimy_die, 1),
//     //(grozavs_lucky_die, 1),
//     (heavenly_kingdom_die, 1),
//     (holy_trinity_die, 1),
//     (kings_die, 1),
//     //(lousy_gamblers_die, 1),
//     //(lu_die, 1),
//     (lucky_die, 1),
//     (mathematicians_die, 1),
//     (mother_of_pearl_die, 1),
//     (odd_die, 6),
//     //(ordinary_die.clone(), 1),
//     //(painted_die, 1),
//     //(painters_die, 3),
//     (pie_die, 6),
//     //(premolar_die, 1),
//     (sad_greasers_die, 1),
//     //(saint_antiochus_die, 1),
//     (shrinking_die, 1),
//     //(st_stephens_die, 1),
//     //(strip_die, 1),
//     //(three_die, 1),
//     (unbalanced_die, 1),
//     //(unlucky_die, 1),
//     //(wagoners_die, 1),
//     (weighted_die, 1),
//     //(wisdom_tooth_die, 1),
// ];



    let all_dice_with_counts = vec![
    (aranka_die, 1),
    (cautious_cheaters_die, 1),
    //(ci_die, 1),
    //(devils_head_die, 1),
    //(die_of_misfortune, 1),
    //(even_die, 1),
    (favourable_die, 1),
    //(fer_die, 1),
    //(greasy_die, 1),
    //(grimy_die, 1),
    //(grozavs_lucky_die, 1),
    //(heavenly_kingdom_die, 1),
    (holy_trinity_die, 3),
    //(kings_die, 1),
    //(lousy_gamblers_die, 1),
    //(lu_die, 1),
    //(lucky_die, 1),
    //(mathematicians_die, 1),
    //(mother_of_pearl_die, 1),
    (odd_die, 3),
    //(ordinary_die.clone(), 1),
    //(painted_die, 1),
    //(painters_die, 3),
    //(pie_die, 1),
    //(sad_greasers_die, 1),
    //(saint_antiochus_die, 1),
    (shrinking_die, 2),
    //(st_stephens_die, 1),
    //(strip_die, 1),
    //(three_die, 1),
    (unbalanced_die, 1),
    (unlucky_die, 1),
    (wagoners_die, 1),
    (weighted_die, 1),
    //(wisdom_tooth_die, 1),
];



//       let all_dice_with_counts = vec![
//     //(aranka_die, 1),
//     //(cautious_cheaters_die, 1),
//     (lu_ci_die, 6),
//     //(devils_head_die, 1),
//     (die_of_misfortune, 6),
//     (even_die, 6),
//     //(favourable_die, 6),
//     (fer_die, 6),
//     (greasy_die, 1),
//     (grimy_die, 1),
//     (grozavs_lucky_die, 6),
//     //(heavenly_kingdom_die, 1),
//     (holy_trinity_die, 6),
//     //(kings_die, 1),
//     (lousy_gamblers_die, 1),
//     (lucky_die, 1),
//     (mathematicians_die, 6),
//     (mother_of_pearl_die, 1),
//     (odd_die, 6),
//     //(ordinary_die.clone(), 1),
//     (painted_die, 1),
//     (painters_die, 3),
//     (pie_die, 6),
//     (sad_greasers_die, 1),
//     (saint_antiochus_die, 1),
//     (shrinking_die, 1),
//     //(st_stephens_die, 1),
//     (strip_die, 1),
//     (three_die, 1),
//     (unbalanced_die, 1),
//     (unlucky_die, 1),
//     (wagoners_die, 6),
//     (weighted_die, 1),
//     //(wisdom_tooth_die, 1),
// ];



//
//     let all_dice_with_counts = vec![
//     //(aranka_die, 1),
//     //(cautious_cheaters_die, 1),
//     //(ci_die, 1),
//     //(devils_head_die, 1),
//     //(die_of_misfortune, 1),
//     //(even_die, 1),
//     (favourable_die, 6),
//     //(fer_die, 1),
//     //(greasy_die, 1),
//     //(grimy_die, 1),
//     //(grozavs_lucky_die, 1),
//     //(heavenly_kingdom_die, 1),
//     //(holy_trinity_die, 1),
//     //(kings_die, 1),
//     //(lousy_gamblers_die, 1),
//     //(lu_die, 1),
//     //(lucky_die, 1),
//     //(mathematicians_die, 1),
//     //(mother_of_pearl_die, 1),
//     //(odd_die, 6),
//     //(ordinary_die.clone(), 1),
//     //(painted_die, 1),
//     //(painters_die, 3),
//     //(pie_die, 6),
//     //(premolar_die, 1),
//     //(sad_greasers_die, 1),
//     //(saint_antiochus_die, 1),
//     //(shrinking_die, 1),
//     //(st_stephens_die, 1),
//     //(strip_die, 1),
//     //(three_die, 1),
//     //(unbalanced_die, 1),
//     //(unlucky_die, 1),
//     //(wagoners_die, 1),
//     (weighted_die, 1),
//     //(wisdom_tooth_die, 1),
// ];

    let (all_dice, all_dice_counts): (Vec<Die>, Vec<u8>) =
    all_dice_with_counts.into_iter().unzip();


//    let available_dice = [default_die.clone(), holy_trinity_die.clone(), aranka_die.clone(), unbalanced_die.clone(),die_of_misfortune.clone(),  shrinking_die.clone(), lu_die.clone(), cautious_cheaters_die.clone(), odd_die.clone(), Die::devil(), weighted_die.clone(), painted_die.clone()];
//    let available_dice_count = [6, 4, 1,1,1,3,1,1,3,0,1,1];



    let (best_score, best_counts) = find_best_dice_set(&all_dice, &all_dice_counts);

    println!(
        "Best = {:.2}  ({})",
        best_score,
        format_dice_set(&all_dice, &best_counts)
    );

let (dice_types, counts) =  selection_from_counts(&all_dice, &best_counts);
    let hist = enumerate_histogram_probabilities(&dice_types, &counts);
    print_score_distribution(&hist);
evaluate_histogram(&dice_types, &counts);

        let hist = enumerate_histogram_probabilities(&[ordinary_die.clone()], &[6]);
    print_score_distribution(&hist);



    /*
let (dice_types, counts) =
    selection_from_counts(&available_dice, &best_counts);
     evaluate_and_print(&dice_types, &counts);
*/

    /*
    // gamma < 1.0 makes low probabilities more visible if distribution is peaky
    render_probability_image_216_from_counts(&dice_types, &counts, "prob_216.png", 0.6);
    println!("Saved prob_216.png");



    let dice_types = [default_die.clone()];
    let counts = [6u8];
    evaluate_and_print(&dice_types, &counts);
    // gamma < 1.0 makes low probabilities more visible if distribution is peaky
    render_probability_image_216_from_counts(&dice_types, &counts, "prob_216_fair.png", 1.0);
    println!("Saved prob_216.png");
  */


}
