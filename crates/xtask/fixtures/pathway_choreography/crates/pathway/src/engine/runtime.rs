fn bad_runtime(self_ref: &mut BadRuntime) {
    self_ref.transport.send_transport(endpoint, payload);
    self_ref.retention.retain_payload(object_id, payload);
    self_ref.effects.record_route_event(event);
}
