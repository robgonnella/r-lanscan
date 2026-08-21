use crate::network::get_default_interface;

use super::*;

#[test]
#[should_panic]
fn should_panic_without_elevated_privileges() {
    let interface = get_default_interface().unwrap();
    let _ = default(&interface).unwrap();
}
