use crate::util::stats::{average_option_u32, average_u32, mode_string as util_mode_string};

pub(super) use crate::util::stats::min_max_spread_u32;

pub(super) fn mean_u32(values: impl Iterator<Item = u32>) -> u32 {
    average_u32(values)
}

pub(super) fn mean_option_u32(values: impl Iterator<Item = Option<u32>>) -> Option<u32> {
    average_option_u32(values)
}

pub(super) fn mode_string(values: impl Iterator<Item = String>) -> String {
    util_mode_string(values).unwrap_or_else(|| "none".to_string())
}

pub(super) fn mode_option_string(values: impl Iterator<Item = Option<String>>) -> Option<String> {
    let collected = values.flatten().collect::<Vec<_>>();
    if collected.is_empty() {
        return None;
    }
    Some(mode_string(collected.into_iter()))
}
