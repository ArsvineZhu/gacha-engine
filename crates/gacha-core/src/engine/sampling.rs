use super::*;

pub(super) fn sample_branch_index(branches: &[Branch], rng: &mut SplitMix64) -> Result<usize> {
    let mut remaining = Rational::ONE;
    for (index, branch) in branches.iter().enumerate() {
        let conditional = branch.probability.checked_div(remaining)?;
        if conditional == Rational::ONE
            || rng.below_u128(conditional.denominator())? < conditional.numerator()
        {
            return Ok(index);
        }
        remaining = remaining.checked_sub(branch.probability)?;
    }
    Err(EngineError::new("branch probabilities do not sum to one"))
}
