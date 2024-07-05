use super::LazyLooper;

#[derive(Debug)]
pub struct PortTargets<'a>(&'a Vec<String>);

pub fn new<'a>(list: &'a Vec<String>) -> PortTargets {
    PortTargets(list)
}

impl<'a> LazyLooper<u32> for PortTargets<'a> {
    fn lazy_loop(self, cb: impl Fn(u32)) {
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
