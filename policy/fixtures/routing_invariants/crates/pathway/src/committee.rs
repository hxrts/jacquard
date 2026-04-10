pub fn committee_selection(current_tick: Tick) -> Tick {
    Tick(current_tick.0 + 12)
}
