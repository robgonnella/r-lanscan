use crate::{
    network,
    packet::mocks::{MockPacketReader, MockPacketSender},
};

use super::*;

#[test]
fn creates_default_wire() {
    let interface = network::get_default_interface().unwrap();
    let wire = default(&interface);
    assert!(wire.is_ok());
}

#[test]
fn returns_packet_result() {
    let mut mock = MockPacketReader::new();
    mock.expect_next_packet().returning(|| Ok(&[1]));
    let result = mock.next_packet();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), &[1]);
}

#[test]
fn send_packet() {
    let mut mock = MockPacketSender::new();
    mock.expect_send()
        .withf(|p| *p == [1])
        .returning(|_| Ok(()));
    let result = mock.send(&[1]);
    assert!(result.is_ok())
}
