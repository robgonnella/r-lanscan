use ratatui::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use std::{thread, time::Duration};

use super::ScrollThrottle;

fn key_event(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[test]
fn first_scroll_event_is_not_throttled() {
    // Each direction's first event should not be throttled
    let throttle_down = ScrollThrottle::default();
    assert!(!throttle_down.throttled(&key_event(KeyCode::Down)));

    let throttle_up = ScrollThrottle::default();
    assert!(!throttle_up.throttled(&key_event(KeyCode::Up)));

    let throttle_j = ScrollThrottle::default();
    assert!(!throttle_j.throttled(&key_event(KeyCode::Char('j'))));

    let throttle_k = ScrollThrottle::default();
    assert!(!throttle_k.throttled(&key_event(KeyCode::Char('k'))));
}

#[test]
fn rapid_scroll_events_are_throttled() {
    let throttle = ScrollThrottle::new(Duration::from_millis(50));

    // First event not throttled
    assert!(!throttle.throttled(&key_event(KeyCode::Down)));

    // Rapid subsequent events are throttled
    assert!(throttle.throttled(&key_event(KeyCode::Down)));
    assert!(throttle.throttled(&key_event(KeyCode::Down)));
}

#[test]
fn scroll_events_after_throttle_duration_are_not_throttled() {
    let throttle = ScrollThrottle::new(Duration::from_millis(20));

    // First event
    assert!(!throttle.throttled(&key_event(KeyCode::Down)));

    // Wait for throttle duration to pass
    thread::sleep(Duration::from_millis(25));

    // Should not be throttled after duration passes
    assert!(!throttle.throttled(&key_event(KeyCode::Down)));
}

#[test]
fn up_and_down_scroll_are_throttled_independently() {
    let throttle = ScrollThrottle::new(Duration::from_millis(50));

    // First down event not throttled
    assert!(!throttle.throttled(&key_event(KeyCode::Down)));

    // Rapid down events are throttled
    assert!(throttle.throttled(&key_event(KeyCode::Down)));

    // But up events can still fire (independent throttle)
    assert!(!throttle.throttled(&key_event(KeyCode::Up)));

    // Now rapid up events are throttled
    assert!(throttle.throttled(&key_event(KeyCode::Up)));
}

#[test]
fn arrow_keys_and_vi_keys_share_same_throttle() {
    let throttle = ScrollThrottle::new(Duration::from_millis(50));

    // Down arrow fires
    assert!(!throttle.throttled(&key_event(KeyCode::Down)));

    // j (vi down) is throttled because down arrow just fired
    assert!(throttle.throttled(&key_event(KeyCode::Char('j'))));

    // Wait for throttle to expire
    thread::sleep(Duration::from_millis(55));

    // Up arrow fires
    assert!(!throttle.throttled(&key_event(KeyCode::Up)));

    // k (vi up) is throttled because up arrow just fired
    assert!(throttle.throttled(&key_event(KeyCode::Char('k'))));
}

#[test]
fn non_scroll_events_are_never_throttled() {
    let throttle = ScrollThrottle::new(Duration::from_millis(50));

    // First down event
    assert!(!throttle.throttled(&key_event(KeyCode::Down)));

    // Non-scroll keys are never throttled, even during throttle window
    assert!(!throttle.throttled(&key_event(KeyCode::Enter)));
    assert!(!throttle.throttled(&key_event(KeyCode::Char('q'))));
    assert!(!throttle.throttled(&key_event(KeyCode::Char('a'))));
    assert!(!throttle.throttled(&key_event(KeyCode::Tab)));
}

#[test]
fn custom_throttle_duration_is_respected() {
    // use longer throttle with shorter sleeps to deal with imprecise ci runners
    let throttle = ScrollThrottle::new(Duration::from_millis(1000));

    // First event
    assert!(!throttle.throttled(&key_event(KeyCode::Down)));

    // After 50ms, still throttled:
    thread::sleep(Duration::from_millis(50));
    assert!(throttle.throttled(&key_event(KeyCode::Down)));

    // Go past throttle to deal with imprecise runners
    thread::sleep(Duration::from_millis(1000));
    assert!(!throttle.throttled(&key_event(KeyCode::Down)));
}

#[test]
fn multiple_throttled_events_dont_reset_timer() {
    // use longer throttle with shorter sleeps to deal with imprecise ci runners
    let throttle = ScrollThrottle::new(Duration::from_millis(1000));

    // First event at T=0
    assert!(!throttle.throttled(&key_event(KeyCode::Down)));

    // Throttled events at T=10, T=20, T=30 shouldn't reset timer
    thread::sleep(Duration::from_millis(10));
    assert!(throttle.throttled(&key_event(KeyCode::Down)));

    thread::sleep(Duration::from_millis(10));
    assert!(throttle.throttled(&key_event(KeyCode::Down)));

    thread::sleep(Duration::from_millis(10));
    assert!(throttle.throttled(&key_event(KeyCode::Down)));

    // Go past throttle to deal with imprecise runners
    thread::sleep(Duration::from_millis(1000));
    assert!(!throttle.throttled(&key_event(KeyCode::Down)));
}
