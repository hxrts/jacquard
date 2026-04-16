use std::collections::BTreeMap;

#[must_use]
pub(crate) fn ratio_permille(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(1000) / denominator
}

#[must_use]
pub(crate) fn average_u32<I>(iter: I) -> u32
where
    I: Iterator<Item = u32>,
{
    let values = iter.collect::<Vec<_>>();
    if values.is_empty() {
        return 0;
    }
    let sum = values
        .iter()
        .fold(0u64, |acc, value| acc.saturating_add(u64::from(*value)));
    u32::try_from(sum / u64::try_from(values.len()).unwrap_or(1)).unwrap_or(u32::MAX)
}

#[must_use]
pub(crate) fn average_option_u32<I>(iter: I) -> Option<u32>
where
    I: Iterator<Item = Option<u32>>,
{
    let values = iter.flatten().collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    Some(average_u32(values.into_iter()))
}

#[must_use]
pub(crate) fn median_u32(values: &[u32]) -> Option<u32> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    Some(sorted[sorted.len() / 2])
}

#[must_use]
pub(crate) fn min_max_spread_u32<I>(iter: I) -> (u32, u32, u32)
where
    I: Iterator<Item = u32>,
{
    let collected = iter.collect::<Vec<_>>();
    let min = collected.iter().copied().min().unwrap_or(0);
    let max = collected.iter().copied().max().unwrap_or(0);
    (min, max, max.saturating_sub(min))
}

#[must_use]
pub(crate) fn mode_string<I>(iter: I) -> Option<String>
where
    I: Iterator<Item = String>,
{
    let mut counts = BTreeMap::new();
    for value in iter {
        *counts.entry(value).or_insert(0u32) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(value, count)| (*count, value.clone()))
        .map(|(value, _)| value)
}
