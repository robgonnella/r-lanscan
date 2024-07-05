use super::LazyLooper;

#[derive(Debug)]
pub struct PortTargets<'a>(&'a Vec<String>);

pub fn new<'a>(list: &'a Vec<String>) -> PortTargets {
    PortTargets(list)
}

impl<'a> LazyLooper<u32> for PortTargets<'a> {
    fn lazy_loop<F: FnMut(u32)>(self, mut cb: F) {
        for target in self.0 {
            if target.contains("-") {
                let parts: Vec<&str> = target.split("-").collect();
                let begin = parts[0].parse::<u32>().unwrap();
                let end = parts[1].parse::<u32>().unwrap();
                for port in begin..end {
                    cb(port)
                }
            } else {
                let port = target.parse::<u32>().unwrap();
                cb(port)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_new_port_targets() {
        let list = vec![String::from("1"), String::from("2"), String::from("3")];
        let targets = new(&list);
        assert_eq!(targets.0, &list);
    }

    #[test]
    fn lazy_loops_ports() {
        let list = vec![String::from("1"), String::from("2-4")];

        let expected = [1, 2, 3, 4];

        let targets = new(&list);

        let mut idx = 0;

        let assert_ports = |port: u32| {
            assert_eq!(port, expected[idx]);
            idx += 1;
        };

        targets.lazy_loop(assert_ports);
    }
}
