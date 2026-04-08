pub fn maybe_select_committee() -> Option<CommitteeSelection> {
    selector.select_committee(&topology).ok().flatten()
}
