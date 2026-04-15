use super::BTreeMap;

pub(super) fn mean_u32(values: impl Iterator<Item = u32>) -> u32 {
    let collected = values.collect::<Vec<_>>();
    if collected.is_empty() {
        return 0;
    }
    let sum = collected.iter().copied().map(u64::from).sum::<u64>();
    u32::try_from(sum / u64::try_from(collected.len()).unwrap_or(1)).unwrap_or(u32::MAX)
}

pub(super) fn mean_option_u32(values: impl Iterator<Item = Option<u32>>) -> Option<u32> {
    let collected = values.flatten().collect::<Vec<_>>();
    if collected.is_empty() {
        return None;
    }
    Some(mean_u32(collected.into_iter()))
}

pub(super) fn mode_string(values: impl Iterator<Item = String>) -> String {
    let mut counts = BTreeMap::<String, u32>::new();
    for value in values {
        *counts.entry(value).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by(|left, right| left.1.cmp(&right.1).then(left.0.cmp(&right.0)))
        .map(|(value, _)| value)
        .unwrap_or_else(|| "none".to_string())
}

pub(super) fn mode_option_string(values: impl Iterator<Item = Option<String>>) -> Option<String> {
    let collected = values.flatten().collect::<Vec<_>>();
    if collected.is_empty() {
        return None;
    }
    Some(mode_string(collected.into_iter()))
}
