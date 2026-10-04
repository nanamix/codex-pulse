#[path = "../src-tauri/src/exit_gate.rs"]
mod exit_gate;
#[test]
fn repeated_exit_requests_wait_for_cleanup() {
    let gate = exit_gate::ExitGate::default();
    assert!(gate.request_cleanup());
    assert!(!gate.can_exit());
    assert!(!gate.request_cleanup());
    assert!(!gate.can_exit());
    gate.finish();
    assert!(gate.can_exit());
    assert!(!gate.request_cleanup());
}
